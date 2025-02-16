use std::io::{self, stdout, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use clap::Parser;
use cli::ui::external::ExternalUIFactory;
use cli::ui::StdoutAnsiTerminalWriteUIFactory;
use cli::{Cli, Commands};

mod app;
mod cli;
mod collections;
mod daemon;
mod logging;
mod net;
mod testbed;
mod transport;

use cli::stdio::StdinReader;
use crossterm::style::{Print, ResetColor};
use logging::KodachiLogger;

async fn run_with<TInput: BufRead, TResponse: 'static + Write + Send>(
    cli: Cli,
    input: TInput,
    response: TResponse,
) -> io::Result<()> {
    match cli.ui() {
        cli::UiConfig::External {
            window_size_provided,
        } => {
            daemon::daemon(
                ExternalUIFactory {
                    will_send_window_size: window_size_provided,
                },
                input,
                response,
            )
            .await
        }

        cli::UiConfig::Stdout => {
            daemon::daemon(StdoutAnsiTerminalWriteUIFactory, input, response).await
        }
    }
}

async fn run(cli: Cli) -> io::Result<()> {
    match &cli.command {
        Commands::Stdio { .. } => {
            let input = StdinReader::stdin();
            let response = io::stderr();
            run_with(cli, input, response).await
        }

        Commands::Unix { path, .. } => {
            let socket = match UnixStream::connect(path) {
                Ok(socket) => socket,
                Err(e) => panic!("Invalid unix socket: {}", e),
            };
            let input = BufReader::new(socket.try_clone().unwrap());
            let response = socket;
            run_with(cli, input, response).await
        }

        Commands::Testbed => panic!(),
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    log::set_boxed_logger(Box::new(KodachiLogger::default())).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    if cli.command == Commands::Testbed {
        return testbed::run();
    }

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(run(cli));

    // Clean up any dangling threads (connections, etc.)
    rt.shutdown_timeout(Duration::from_millis(100));

    // Leave some room to print the error clearly
    ::crossterm::execute!(stdout(), ResetColor, Print("\n\n"))?;

    result
}
