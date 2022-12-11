use std::io;

use bytes::Bytes;

pub mod telnet;

pub enum TransportEvent {
    Data(Bytes),
    Nop,
}

pub trait Transport {
    fn read(&mut self) -> io::Result<TransportEvent>;
    fn write(&mut self, data: &[u8]) -> io::Result<usize>;
}
