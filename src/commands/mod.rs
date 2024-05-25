use clap::{Parser, Subcommand};

use crate::utils::{DEFAULT_RELAY_HOST, DEFAULT_RELAY_PORT};

pub mod send_command;
pub mod get_command;
pub mod server_command;

#[derive(Parser, Debug)]
#[clap(version = "0.1", author = "darmiel <asdf@qwer.tz>")]
pub struct RootOpts {
    #[clap(short = 'x', long, default_value = DEFAULT_RELAY_HOST)]
    pub(crate) relay_host: String,

    #[clap(short = 'y', long, default_value = DEFAULT_RELAY_PORT)]
    pub(crate) relay_port: u16,

    #[clap(short, long, default_value = "false")]
    pub(crate) verbose: bool,

    #[clap(subcommand)]
    pub(crate) subcmd: SubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    Serve(server_command::RelayServerOpts),
    Send(send_command::SendOpts),
    Get(get_command::GetOpts),
}