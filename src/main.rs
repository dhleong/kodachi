use std::{
    env,
    io::{self, BufRead, Read, Stdin},
};

mod connection;
mod daemon;
mod transport;

struct StdinReader(Stdin);

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let mut args = env::args_os().skip(1);
    let uri = args.next();
    if let Some(uri) = uri {
        tokio::spawn(connection::connection(
            uri.to_string_lossy().to_string(),
            5656,
        ))
        .await??;
    } else {
        let stdin = StdinReader(io::stdin());
        tokio::spawn(daemon::daemon(stdin)).await??;
    }
    Ok(())
}
