use std::ops::Deref;

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

    pub fn strip_ansi(&mut self) -> AnsiStripped {
        if let Some(existing) = self.stripped.as_ref() {
            return existing.clone();
        }

        // TODO Use Bytes ranges from self.bytes to avoid excessive copying
        let raw = std::str::from_utf8(&self.bytes).unwrap();
        let mut without_ansi = String::new();
        let mut maybe_csi = false;
        let mut in_csi = false;

        for ch in raw.chars() {
            match (ch as u8, maybe_csi, in_csi) {
                // ESC
                (0x1bu8, false, false) => {
                    maybe_csi = true;
                    in_csi = false;
                }

                // [
                (0x5Bu8, true, false) => {
                    maybe_csi = false;
                    in_csi = true;
                }

                // Detect ending
                (as_byte, false, true) => {
                    if (0x40..0x7E).contains(&as_byte) {
                        in_csi = false;
                    }
                }

                _ => {
                    without_ansi.push(ch);
                }
            };
        }

        // Cache the result:
        let stripped = AnsiStripped {
            value: Bytes::from(without_ansi),
        };
        self.stripped = Some(stripped.clone());
        return stripped;
    }
}

#[derive(Clone)]
pub struct AnsiStripped {
    value: Bytes,
    // TODO Ranges for mapping back to Ansi bytes
}

impl Deref for AnsiStripped {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        std::str::from_utf8(&self.value).unwrap()
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
}
