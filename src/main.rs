use std::io::{self, BufRead, Read, Stdin};

use clap::StructOpt;
use cli::{Cli, Commands};
mod app;
mod cli;
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

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Stdio => {
            let input = StdinReader(io::stdin());
            let response = io::stderr();
            daemon::daemon(input, response).await?;
        }
    }

    Ok(())
}
