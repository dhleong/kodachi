use std::ops::Deref;

use bytes::{BufMut, BytesMut};

#[derive(Clone, Default)]
pub struct Ansi(BytesMut);

impl Deref for Ansi {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // TODO strip ANSI codes
        std::str::from_utf8(&self.0).unwrap()
    }
}

impl AsRef<[u8]> for Ansi {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Into<BytesMut> for Ansi {
    fn into(self) -> BytesMut {
        self.0
    }
}

impl From<&str> for Ansi {
    fn from(source: &str) -> Self {
        Self::from_bytes(BytesMut::from(source))
    }
}

impl Ansi {
    pub fn from_bytes(bytes: BytesMut) -> Self {
        Self(bytes)
    }

    pub fn from<T: Into<BytesMut>>(bytes: T) -> Self {
        Self::from_bytes(bytes.into())
    }

    pub fn into_inner(self) -> BytesMut {
        self.0
    }

    pub fn put_slice(&mut self, bytes: &[u8]) {
        self.0.put_slice(bytes)
    }

    pub fn take(&mut self) -> BytesMut {
        let result = self.0.clone();

        self.0.clear();

        return result;
    }
}
