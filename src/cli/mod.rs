use clap::{Parser, Subcommand, ValueEnum};

pub mod stdio;
pub mod ui;

#[derive(Parser)]
#[clap(author, version, about, propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,

    #[arg(long, value_enum, global = true, default_value = "stdout")]
    pub ui: UiType,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum UiType {
    Stdout,

    External,
}

#[derive(Subcommand, PartialEq)]
pub enum Commands {
    /// Run in daemon mode using stdio streams
    Stdio,

    /// Run in daemon mode using a unix domain socket
    Unix { path: String },

    #[clap(hide = true)]
    Testbed,
}
