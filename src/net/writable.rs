use std::io::{self, Write};

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[async_trait]
pub trait ObjectWriteStream {
    async fn write_object<T: Writable + Send>(&mut self, object: T) -> io::Result<()>;
}

pub trait Writable {
    fn write<S: Write>(self, stream: &mut S) -> io::Result<()>;

    fn into_bytes(self) -> Bytes
    where
        Self: Sized,
    {
        let mut writer = BytesMut::default().writer();
        self.write(&mut writer)
            .expect("Unexpected IO error writing to BytesMut");
        writer.into_inner().freeze()
    }
}

#[async_trait]
impl<T: AsyncWrite + Unpin + Send> ObjectWriteStream for T {
    async fn write_object<O: Writable + Send>(&mut self, object: O) -> io::Result<()> {
        let mut bytes = object.into_bytes();
        self.write_all_buf(&mut bytes).await
    }
}
