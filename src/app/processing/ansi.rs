use std::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::{Add, Deref, Range, RangeBounds},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};

// Represents a mutable Bytes string containing Ansi sequences. Because it is mutable,
// and expected to be used for *constructing* Ansi instances, it *may* contain invalid
// ansi or utf8 sequences.
#[derive(Clone, Debug, Default)]
pub struct AnsiMut(BytesMut);

/// Converts as much of the input type to utf8 as is valid to do.
fn valid_utf8_bytes_count<T: AsRef<[u8]> + Debug>(s: &T) -> usize {
    match std::str::from_utf8(s.as_ref()) {
        Ok(_) => s.as_ref().len(),
        Err(error) => error.valid_up_to(),
    }
}

fn maximally_as_utf8<T: AsRef<[u8]> + Debug>(s: &T) -> &str {
    std::str::from_utf8(&s.as_ref()[0..valid_utf8_bytes_count(s)])
        .map_err(|err| format!("Error parsing {s:?} into utf8: {err:?}"))
        .unwrap()
}

impl AsRef<[u8]> for AnsiMut {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<AnsiMut> for BytesMut {
    fn from(val: AnsiMut) -> Self {
        val.0
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

    pub fn into_inner(self) -> BytesMut {
        self.0
    }

    /// "Deref" this AnsiMut and get as much valid utf8 str as is available.
    /// NOTE: This MAY NOT represent the entire contents of this AnsiMut instance, since
    /// we could be still pending more bytes to "complete" utf8 sequences at the end
    pub fn valid_utf8(&self) -> &str {
        maximally_as_utf8(&self.0)
    }

    pub fn put_slice(&mut self, bytes: &[u8]) {
        self.0.put_slice(bytes)
    }

    pub fn take(&mut self) -> Ansi {
        let cnt = valid_utf8_bytes_count(&self.0);
        if cnt == self.0.len() {
            // Valid utf8
            let bytes = self.0.split();
            Ansi::from_bytes(bytes.freeze())
        } else if self.0.iter().skip(cnt).any(|c| *c == b'\n') {
            // Invalid utf8
            let bytes = self.0.split();
            let lossy = String::from_utf8_lossy(&bytes);
            Ansi::from_bytes(Bytes::from(lossy.to_string()))
        } else {
            // Possibly incomplete utf8; only take what's valid
            let bytes = self.0.split_to(cnt);
            Ansi::from_bytes(bytes.freeze())
        }
    }

    /// Drop the `count` bytes at the beginning of this AnsiMut instance (IE: `[0, count)`) and
    /// return them. This instance retains the bytes from `[count, ...]`
    pub fn drop_leading_bytes(&mut self, count: usize) -> BytesMut {
        self.0.split_to(count)
    }

    pub fn has_incomplete_code(&self) -> bool {
        let cnt = valid_utf8_bytes_count(&self.0);
        if cnt < self.0.len() {
            if self.0.iter().position(|c| *c == b'\n') > Some(cnt) {
                // Probably *not* incomplete utf8, but rather *invalid* utf8.
                return false;
            }
            return true;
        }

        // Would be nice if this didn't require so much copying:
        let arr: &[u8] = &self.0;
        let bytes = (&arr[..]).copy_to_bytes(arr.len());
        strip_ansi(bytes).has_incomplete
    }
}

// Represents a Bytes string containing *valid* Ansi sequences. Is NOT expected to contain any
// partial Ansi or utf8 sequences.
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
        // NOTE: We should already have stripped invalid utf8 sequences when converting to
        // Ansi from AnsiMut; Ansi should never contain invalid utf8 sequences!
        std::str::from_utf8(&self.bytes).unwrap()
    }
}

impl AsRef<[u8]> for Ansi {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.bytes.as_ref()
    }
}

impl From<&'static str> for Ansi {
    fn from(source: &'static str) -> Self {
        Ansi::from_bytes(Bytes::from_static(source.as_bytes()))
    }
}

impl From<String> for Ansi {
    fn from(source: String) -> Self {
        Ansi::from_bytes(Bytes::from(source))
    }
}

impl From<Ansi> for Bytes {
    fn from(val: Ansi) -> Self {
        val.bytes
    }
}

impl Add for Ansi {
    type Output = Ansi;

    fn add(self, rhs: Self) -> Self::Output {
        // Avoid copying if one or the other is empty
        if self.bytes.is_empty() {
            rhs
        } else if rhs.bytes.is_empty() {
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

    fn from_bytes(bytes: Bytes) -> Self {
        Self {
            bytes,
            stripped: None,
        }
    }

    pub fn as_bytes(&self) -> Bytes {
        self.bytes.clone()
    }

    pub fn into_inner(self) -> Bytes {
        self.bytes
    }

    /// This is a very niche method designed to avoid exposing a potentially dangerous
    /// "unbounded slice" method, while being as efficient as possible. Here, the caller
    /// is attesting that match_range is a range within stripped, which came from calling
    /// strip_ansi on this Ansi instance.
    pub fn without_stripped_match_range(
        &self,
        stripped: &AnsiStripped,
        match_range: Range<usize>,
    ) -> Ansi {
        let consumed_range = stripped.get_original_range(match_range);
        self.slice(0..consumed_range.start) + self.slice(consumed_range.end..self.bytes.len())
    }

    fn slice(&self, range: impl RangeBounds<usize>) -> Ansi {
        Ansi::from_bytes(self.bytes.slice(range))
    }

    pub fn strip_ansi(&mut self) -> AnsiStripped {
        if let Some(existing) = self.stripped.as_ref() {
            return existing.clone();
        }

        // Cache the result:
        let stripped = strip_ansi(self.bytes.clone());
        self.stripped = Some(stripped.clone());
        stripped
    }

    // Returns an Ansi instance, which shares its backing data with this Ansi instance, but whose
    // accessible range does not have include any trailing newlines
    pub fn trim_trailing_newlines(&self) -> Ansi {
        if !self.ends_with(['\r', '\n']) {
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
        self.clone()
    }
}

fn strip_ansi(bytes: Bytes) -> AnsiStripped {
    // NOTE: It'd be nice if we could reuse Bytes ranges from self.bytes to avoid excessive
    // copying---esp if there is actually no Ansi in self.bytes
    // NOTE: We not need maximally_as_utf8 here, as we are only called from Ansi, which *must*
    // have valid utf8 bytes.
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
        let s = maximally_as_utf8(&self.value);
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

        let ansi = Ansi::from("grayskull\n");
        assert_eq!(&ansi.trim_trailing_newlines()[..], "grayskull");

        let ansi = Ansi::from("grayskull\r\r\n");
        assert_eq!(&ansi.trim_trailing_newlines()[..], "grayskull");
    }

    #[test]
    #[allow(clippy::bool_assert_comparison)] // I think it's slightly more readable here
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

    #[test]
    fn deref_ansi_mut_utf8_safely() {
        // NOTE: This is likely an incomplete sequence of some kind, eg: \xe2\x96\x84
        let bytes: &[u8] = b"\r\xe2";
        let mut ansi_mut = AnsiMut::from_bytes(BytesMut::from(bytes));
        assert!(ansi_mut.has_incomplete_code());

        let ansi = ansi_mut.take();
        assert!(ansi.starts_with("\r"));

        // NOTE: The AnsiMut instance retains the invalid utf8 bytes
        assert_eq!(ansi_mut.as_ref(), b"\xe2");

        // If we "finish" the utf8 sequence, we can now take it:
        ansi_mut.put_slice(b"\x96\x84");
        let remainder = ansi_mut.take();
        assert_eq!(remainder.as_bytes().as_ref(), b"\xe2\x96\x84");

        // ... and the AnsiMut will be empty
        assert!(ansi_mut.into_inner().is_empty());
    }

    #[cfg(test)]
    mod stripped_ansi {
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
