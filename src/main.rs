use std::io::{self, BufReader};
use std::os::unix::net::UnixStream;

use clap::StructOpt;
use cli::{Cli, Commands};

mod app;
mod cli;
mod daemon;
mod net;
mod transport;

use cli::stdio::StdinReader;

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Stdio => {
            let input = StdinReader::stdin();
            let response = io::stderr();
            daemon::daemon(input, response).await?;
        }

        Commands::Unix { path } => {
            let socket = match UnixStream::connect(path) {
                Ok(socket) => socket,
                Err(e) => panic!("Invalid unix socket: {}", e),
            };
            let input = BufReader::new(socket.try_clone().unwrap());
            let response = socket;
            daemon::daemon(input, response).await?;
        }
    }

    Ok(())
}
