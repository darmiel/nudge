#[macro_use]
extern crate simple_log;

use clap::{Parser};
use simple_log::LogConfigBuilder;

use crate::error::Result;
use crate::commands::{SubCommand, server_command, send_command, get_command};

mod error;
mod utils;

// subcommands
mod commands;
mod models;


fn main() -> Result<()> {
    let opts = commands::RootOpts::parse();

    // init logger
    let log_config = LogConfigBuilder::builder()
        .level(if opts.verbose {
            log::Level::Debug.as_str()
        } else {
            log::Level::Info.as_str()
        })
        .time_format("%d-%m/%H:%M:%S")
        .output_console()
        .build();
    simple_log::new(log_config).expect("Failed to initialize logger");

    match match &opts.subcmd {
        SubCommand::Serve(server_opts) => server_command::run(&opts, &server_opts),
        SubCommand::Send(send_opts) => send_command::run(&opts, &send_opts),
        SubCommand::Get(get_opts) => get_command::run(&opts, &get_opts),
    } {
        Err(e) => {
            error!("Error: {}", e);
            Err(e)
        }
        _ => Ok(()),
    }
}