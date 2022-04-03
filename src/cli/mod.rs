use clap::{Parser, Subcommand};

pub mod stdio;

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

    /// Run in daemon mode using a unix domain socket
    Unix { path: String },
}
