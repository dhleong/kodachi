use std::io;

use ::telnet::Event;

pub mod telnet;

pub trait Transport {
    fn read(&mut self) -> io::Result<Event>;
    fn write(&mut self, data: &[u8]) -> io::Result<usize>;
}
