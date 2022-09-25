use std::ops::{Deref, Range};

use bytes::{BufMut, Bytes, BytesMut};

#[derive(Clone, Default)]
pub struct AnsiMut(BytesMut);

impl Deref for AnsiMut {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        std::str::from_utf8(&self.0).unwrap()
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

impl From<&str> for AnsiMut {
    fn from(source: &str) -> Self {
        Self::from_bytes(BytesMut::from(source))
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
        Ansi::from(bytes.freeze())
    }
}

#[derive(Clone)]
pub struct Ansi {
    bytes: Bytes,
    stripped: Option<AnsiStripped>,
}

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

impl Into<Bytes> for Ansi {
    fn into(self) -> Bytes {
        self.bytes
    }
}

impl Ansi {
    pub fn empty() -> Self {
        Self::from_bytes(Bytes::default())
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

    pub fn into_inner(self) -> Bytes {
        self.bytes
    }

    pub fn iter(&self) -> std::slice::Iter<u8> {
        self.bytes.iter()
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
    }
}

#[derive(Clone)]
pub struct AnsiStripped {
    value: Bytes,
    original: Bytes,
    ansi_ranges: Vec<Range<usize>>,
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
