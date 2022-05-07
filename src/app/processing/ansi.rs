use std::ops::Deref;

use bytes::{BufMut, BytesMut};

#[derive(Clone, Default)]
pub struct Ansi(BytesMut);

impl Deref for Ansi {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        // TODO Strip ANSI codes
        ""
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

impl Ansi {
    pub fn from_bytes(bytes: BytesMut) -> Self {
        Self(bytes)
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
