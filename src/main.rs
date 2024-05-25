#[macro_use]
extern crate simple_log;

use clap::{Parser, Subcommand};
use simple_log::LogConfigBuilder;

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
    #[clap(short, long, default_value = "false")]
    verbose: bool,

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

    // init logger
    let log_config = LogConfigBuilder::builder()
        .level(if opts.verbose {
            log::Level::Debug.as_str()
        } else {
            log::Level::Info.as_str()
        })
        .output_console()
        .build();
    simple_log::new(log_config).expect("Failed to initialize logger");

    match match opts.subcmd {
        SubCommand::Serve(server_opts) => {
            let mut server = server::RelayServer::new(server_opts)?;
            Ok(server.run().expect("Failed to start server"))
        }
        SubCommand::Send(send) => send.run(),
        SubCommand::Get(get) => get.run(),
    } {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Error: {}", e);
            Err(e)
        }
    }
}