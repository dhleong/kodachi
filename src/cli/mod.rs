use clap::{Parser, Subcommand, ValueEnum};

pub mod stdio;
pub mod ui;

#[derive(Parser)]
#[clap(author, version, about, propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,

    #[deprecated = "Prefer using the ui subcommand"]
    #[arg(long, value_enum, global = true, hide = true)]
    pub ui: Option<UiType>,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum UiType {
    Stdout,

    External,
}

#[derive(Subcommand, Clone, PartialEq)]
pub enum UiConfig {
    Stdout,

    External {
        #[arg(long, requires("ui"))]
        window_size_provided: bool,
    },
}

#[derive(Subcommand, PartialEq)]
pub enum Commands {
    /// Run in daemon mode using stdio streams
    Stdio {
        #[clap(subcommand)]
        ui: Option<UiConfig>,
    },

    /// Run in daemon mode using a unix domain socket
    Unix {
        path: String,

        #[clap(subcommand)]
        ui: Option<UiConfig>,
    },

    #[clap(hide = true)]
    Testbed,
}

impl Commands {
    pub fn ui(&self) -> UiConfig {
        match self {
            Self::Stdio { ui } => ui.clone().unwrap_or(UiConfig::Stdout),
            Self::Unix { ui, .. } => ui.clone().unwrap_or(UiConfig::Stdout),
            Self::Testbed => panic!("Testbed doesn't support UI config"),
        }
    }
}

impl Cli {
    pub fn ui(&self) -> UiConfig {
        #[allow(deprecated)]
        match self.ui {
            Some(UiType::Stdout) => UiConfig::Stdout,
            Some(UiType::External) => UiConfig::External {
                window_size_provided: false,
            },
            None => self.command.ui(),
        }
    }
}
