mod build;
mod index;
mod keygen;

use clap::Parser;

use crate::cli::{Cli, Command};

pub(crate) fn run() -> Result<(), String> {
    match Cli::parse().command {
        Command::Keygen(args) => keygen::run(args),
        Command::Build(args) => build::run(args),
        Command::Index(args) => index::run(args),
    }
}
