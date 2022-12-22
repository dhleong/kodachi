use std::io::{self, BufRead};

pub trait Readable {
    fn read<S: BufRead>(stream: &mut S) -> io::Result<Self>
    where
        Self: Sized;
}
