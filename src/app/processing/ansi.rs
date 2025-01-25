use std::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::{Add, Deref, Range, RangeBounds},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};

#[derive(Clone, Default)]
pub struct AnsiMut(BytesMut);

impl Deref for AnsiMut {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        std::str::from_utf8(&self.0)
            .map_err(|err| format!("Error parsing {:?} into utf8: {:?}", self.0, err))
            .unwrap()
    }
}

impl AsRef<[u8]> for AnsiMut {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Into<BytesMut> for AnsiMut {
    fn into(self) -> BytesMut {
        self.0
    }
}

impl Into<Ansi> for AnsiMut {
    fn into(self) -> Ansi {
        Ansi::from_bytes(self.0.freeze())
    }
}

impl<T: AsRef<str>> From<T> for AnsiMut {
    fn from(source: T) -> Self {
        Self::from_bytes(BytesMut::from(source.as_ref()))
    }
}

impl AnsiMut {
    pub fn from_bytes(bytes: BytesMut) -> Self {
        Self(bytes)
    }

    pub fn from<T: Into<BytesMut>>(bytes: T) -> Self {
        Self::from_bytes(bytes.into())
    }

    pub fn put_slice(&mut self, bytes: &[u8]) {
        self.0.put_slice(bytes)
    }

    pub fn take_bytes(&mut self) -> BytesMut {
        self.0.split()
    }

    pub fn take(&mut self) -> Ansi {
        let bytes = self.take_bytes();
        Ansi::from_bytes(bytes.freeze())
    }

    pub fn has_incomplete_code(&self) -> bool {
        // Would be nice if this didn't require so much copying:
        let arr: &[u8] = &self.0;
        let bytes = (&arr[..]).copy_to_bytes(arr.len());
        strip_ansi(bytes).has_incomplete
    }
}

#[derive(Clone)]
pub struct Ansi {
    bytes: Bytes,
    stripped: Option<AnsiStripped>,
}

impl Default for Ansi {
    fn default() -> Self {
        Self::empty()
    }
}

impl Debug for Ansi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.bytes.fmt(f)
    }
}

impl Hash for Ansi {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state)
    }
}

impl PartialEq for Ansi {
    fn eq(&self, other: &Self) -> bool {
        self.bytes.eq(&other.bytes)
    }
}

impl Eq for Ansi {}

impl Deref for Ansi {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        std::str::from_utf8(&self.bytes).unwrap()
    }
}

impl AsRef<[u8]> for Ansi {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl From<&str> for Ansi {
    fn from(source: &str) -> Self {
        AnsiMut::from(source).into()
    }
}

impl From<String> for Ansi {
    fn from(source: String) -> Self {
        source.as_str().into()
    }
}

impl From<BytesMut> for Ansi {
    fn from(source: BytesMut) -> Self {
        Ansi::from_bytes(source.into())
    }
}

impl Into<Bytes> for Ansi {
    fn into(self) -> Bytes {
        self.bytes
    }
}

impl Add for Ansi {
    type Output = Ansi;

    fn add(self, rhs: Self) -> Self::Output {
        // Avoid copying if one or the other is empty
        if self.bytes.len() == 0 {
            rhs
        } else if rhs.bytes.len() == 0 {
            self
        } else {
            let len = self.bytes.len() + rhs.bytes.len();
            let mut chain = self.bytes.chain(rhs.bytes);
            Self::from_bytes(chain.copy_to_bytes(len))
        }
    }
}

impl Ansi {
    pub fn empty() -> Self {
        Self::from("")
    }

    pub fn from_bytes(bytes: Bytes) -> Self {
        Self {
            bytes,
            stripped: None,
        }
    }

    pub fn from<T: Into<Bytes>>(bytes: T) -> Self {
        Self::from_bytes(bytes.into())
    }

    pub fn as_bytes(&self) -> Bytes {
        self.bytes.clone()
    }

    pub fn into_inner(self) -> Bytes {
        self.bytes
    }

    pub fn slice(&mut self, range: impl RangeBounds<usize>) -> Ansi {
        Ansi::from_bytes(self.bytes.slice(range))
    }

    pub fn strip_ansi(&mut self) -> AnsiStripped {
        if let Some(existing) = self.stripped.as_ref() {
            return existing.clone();
        }

        // Cache the result:
        let stripped = strip_ansi(self.bytes.clone());
        self.stripped = Some(stripped.clone());
        return stripped;
    }

    // Returns an Ansi instance, which shares its backing data with this Ansi instance, but whose
    // accessible range does not have include any trailing newlines
    pub fn trim_trailing_newlines(&self) -> Ansi {
        if !self.ends_with(&['\r', '\n'][..]) {
            return self.clone();
        }

        if let Some(end) = self.bytes.len().checked_sub(1) {
            for i in (0..end).rev() {
                let byte = *self
                    .bytes
                    .get(i)
                    .expect("Couldn't access expected index into Bytes");
                if byte != b'\r' && byte != b'\n' {
                    let range = 0..(i + 1);
                    return Ansi::from_bytes(self.bytes.slice(range));
                }
            }
        }
        return self.clone();
    }
}

fn strip_ansi(bytes: Bytes) -> AnsiStripped {
    // NOTE: It'd be nice if we could reuse Bytes ranges from self.bytes to avoid excessive
    // copying---esp if there is actually no Ansi in self.bytes
    let raw = std::str::from_utf8(&bytes).unwrap();
    let mut without_ansi = String::new();
    let mut ansi_ranges = Vec::new();

    let mut maybe_csi = false;
    let mut in_csi = false;
    let mut range_start = 0usize;

    for (index, ch) in raw.char_indices() {
        match (ch as u8, maybe_csi, in_csi) {
            // ESC
            (0x1b, false, false) => {
                maybe_csi = true;
                in_csi = false;
                range_start = index;
            }

            (b'[', true, false) => {
                maybe_csi = false;
                in_csi = true;
            }

            // Detect ending
            (as_byte, false, true) => {
                if (0x40..0x7E).contains(&as_byte) {
                    in_csi = false;
                    ansi_ranges.push(range_start..index + 1);
                }
            }

            _ => {
                without_ansi.push(ch);
            }
        };
    }

    AnsiStripped {
        value: Bytes::from(without_ansi),
        original: bytes,
        ansi_ranges,
        has_incomplete: in_csi || maybe_csi,
    }
}

#[derive(Clone)]
pub struct AnsiStripped {
    value: Bytes,
    original: Bytes,
    ansi_ranges: Vec<Range<usize>>,
    has_incomplete: bool,
}

impl Debug for AnsiStripped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl Display for AnsiStripped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = std::str::from_utf8(&self.value).unwrap();
        Display::fmt(s, f)
    }
}

impl Deref for AnsiStripped {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        std::str::from_utf8(&self.value).unwrap()
    }
}

impl AnsiStripped {
    pub fn get_original(&self, range: Range<usize>) -> Ansi {
        // TODO: It *may* behoove us to grab any ANSI ranges preceeding the `mapped`
        // range, to ensure that the resulting slice is styled as expected...
        let mapped = self.get_original_range(range);
        Ansi::from_bytes(self.original.slice(mapped))
    }

    pub fn get_original_range(&self, range: Range<usize>) -> Range<usize> {
        let mut start = range.start;
        let mut end = range.end;
        for candidate in &self.ansi_ranges {
            if candidate.start < start {
                start += candidate.len();
                end += candidate.len();
            } else if candidate.start <= end {
                end += candidate.len();
            } else {
                break;
            }
        }
        start..end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi() {
        let mut ansi = Ansi::from("\x1b[32mColorful\x1b[m");
        assert_eq!(&ansi.strip_ansi()[..], "Colorful");
    }

    #[test]
    fn but_only_strip_ansi() {
        let mut ansi = Ansi::from("say ['anything']");
        assert_eq!(&ansi.strip_ansi()[..], "say ['anything']");
    }

    #[test]
    fn trim_trailing_newlines() {
        let ansi = Ansi::from("grayskull\r\n");
        assert_eq!(&ansi.trim_trailing_newlines()[..], "grayskull");
    }

    #[test]
    fn detect_incomplete_ansi_codes() {
        assert_eq!(AnsiMut::from("grayskull\x1b").has_incomplete_code(), true);
        assert_eq!(AnsiMut::from("grayskull\x1b[").has_incomplete_code(), true);
        assert_eq!(AnsiMut::from("grayskull\x1b[3").has_incomplete_code(), true);
        assert_eq!(
            AnsiMut::from("grayskull\x1b[32").has_incomplete_code(),
            true
        );
        assert_eq!(
            AnsiMut::from("grayskull\x1b[32m").has_incomplete_code(),
            false
        );
    }

    #[cfg(test)]
    mod striped_ansi {
        use super::*;

        #[test]
        fn maps_back_to_original_at_ansi() {
            let mut ansi = Ansi::from("\x1b[32mEverything\x1b[m is \x1b[32mFine\x1b[m");
            let stripped = ansi.strip_ansi();
            assert_eq!(stripped.get_original_range(0..10), 0..18);

            let original = stripped.get_original(0..10);
            assert_eq!(&original[..], "\x1b[32mEverything\x1b[m");
        }

        #[test]
        fn maps_back_to_original_after_ansi() {
            let mut ansi = Ansi::from("\x1b[32mEverything\x1b[m is \x1b[32mFine\x1b[m");
            let stripped = ansi.strip_ansi();
            assert_eq!(stripped.get_original_range(1..10), 6..18);

            let original = stripped.get_original(1..10);
            assert_eq!(&original[..], "verything\x1b[m");
        }

        #[test]
        fn fully_maps_back_to_original() {
            let mut ansi = Ansi::from("\x1b[32mEverything\x1b[m is \x1b[32mFine\x1b[m");
            let stripped = ansi.strip_ansi();
            assert_eq!(&stripped[..], "Everything is Fine");
            assert_eq!(stripped.get_original_range(0..18), 0..34);

            let original = stripped.get_original(0..18);
            assert_eq!(
                &original[..],
                "\x1b[32mEverything\x1b[m is \x1b[32mFine\x1b[m"
            );
        }
    }
}
