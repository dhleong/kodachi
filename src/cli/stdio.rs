use std::io::{self, BufRead, Read, Stdin};

pub struct StdinReader(Stdin);

impl StdinReader {
    pub fn stdin() -> StdinReader {
        Self(io::stdin())
    }
}

impl Read for StdinReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.lock().read(buf)
    }
}

impl BufRead for StdinReader {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        self.0.lock().read_line(buf)
    }

    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        panic!("Not supported");
    }

    fn consume(&mut self, amt: usize) {
        self.0.lock().consume(amt)
    }
}
