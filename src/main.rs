use clap::{Parser, Subcommand};
use error::Result;

// lib
mod error;
mod utils;
mod passphrase;
mod reliable_udp;

// subcommands
mod server;
mod send;
mod get;
mod models;

#[derive(Parser, Debug)]
#[clap(version = "0.1", author = "darmiel <asdf@qwer.tz>")]
// #[clap(setting = AppSettings::SubcommandRequiredElseHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    Serve(server::RelayServerOpts),
    Send(send::Send),
    Get(get::Get),
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Serve(server_opts) => {
            let mut server = server::RelayServer::new(server_opts)?;
            Ok(server.run().expect("Failed to start server"))
        }
        SubCommand::Send(send) => send.run(),
        SubCommand::Get(get) => get.run(),
    }
}