use std::io;

use async_trait::async_trait;
use bytes::Bytes;

pub mod telnet;

pub enum TransportEvent {
    Data(Bytes),
    Nop,
}

#[async_trait]
pub trait Transport {
    async fn read(&mut self) -> io::Result<TransportEvent>;
    async fn write(&mut self, data: &[u8]) -> io::Result<usize>;
}
