use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run in daemon mode using stdio streams
    Stdio,
}
