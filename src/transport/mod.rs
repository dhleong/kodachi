use std::io;

pub mod telnet;

pub trait Transport {
    fn read(&mut self) -> io::Result<Option<Box<[u8]>>>;
}
