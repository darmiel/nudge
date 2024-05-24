use clap::{Parser, Subcommand};
use error::NudgeError;

// lib
mod error;
mod utils;
mod passphrase;
mod reliable_udp;

// subcommands
mod server;
mod send;
mod get;

#[derive(Parser, Debug)]
#[clap(version = "0.1", author = "darmiel <asdf@qwer.tz>")]
// #[clap(setting = AppSettings::SubcommandRequiredElseHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    Serve(server::RelayServer),
    Send(send::Send),
    Get(get::Get),
}

fn main() -> Result<(), NudgeError> {
    let opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Serve(server) => server.run(),
        SubCommand::Send(send) => send.run(),
        SubCommand::Get(get) => get.run(),
    }
}