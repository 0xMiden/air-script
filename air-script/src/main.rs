use std::io::Write;

use clap::{Parser, Subcommand};

mod cli;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Transpile AirScript source code to Rust targeting Winterfell
    Transpile(cli::Transpile),
}

pub fn main() {
    env_logger::Builder::new()
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .filter_level(log::LevelFilter::Debug)
        .init();

    let cli = Cli::parse();

    let res = match cli.command {
        Command::Transpile(transpile) => transpile.execute(),
    };

    if let Err(error) = res {
        println!("{error}");
    }
}
