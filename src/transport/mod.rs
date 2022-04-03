use std::io;

use ::telnet::TelnetEvent;

pub mod telnet;

pub trait Transport {
    fn read(&mut self) -> io::Result<TelnetEvent>;
    fn write(&mut self, data: &[u8]) -> io::Result<usize>;
}
