use std::fs::File;
use std::io::Read;
use std::net::{Ipv4Addr, UdpSocket};

use clap::Parser;
use console::style;
use humansize::{DECIMAL, format_size};
use crate::commands::RootOpts;

use crate::error::NudgeError;
use crate::models::X2SPassphraseProvidedMessage;
use crate::models::S2XRequestPassphraseMessage;
use crate::models::X2SSenderConnectToReceiverMessage;
use crate::utils::reliable_udp::ReliableUdpSocket;
use crate::utils::AnonymousString;
use crate::utils::current_unix_millis;
use crate::utils::hash_file_and_seek;
use crate::utils::hide_or_get_hostname;
use crate::utils::init_socket;
use crate::utils::new_downloader_progressbar;
use crate::utils::receive_and_parse_and_expect;
use crate::utils::serialize_and_send;
use crate::utils::DEFAULT_CHUNK_SIZE;

#[derive(Parser, Debug)]
pub struct SendOpts {
    file: String,

    #[clap(short, long, default_value = "500")]
    delay: u64,

    #[clap(short, long, default_value = DEFAULT_CHUNK_SIZE)]
    chunk_size: u32,

    /// If enabled, won't send the hostname to the receiver
    #[clap(long, default_value = "false")]
    hide_hostname: bool,

    /// If enabled, won't create a hash of the file
    #[clap(long, default_value = "false")]
    skip_hash: bool,
}

pub fn run(root_opts: &RootOpts, send_opts: &SendOpts) -> Result<(), NudgeError> {
    // check if the file exists and open it
    let mut file = File::open(&send_opts.file)?;
    let file_name = &send_opts.file.split('/').last().unwrap();
    let file_size = file.metadata()?.len();

    let local_bind_address = (Ipv4Addr::from(0u32), 0);
    debug!("Binding UDP socket to local address: {:?}", local_bind_address);
    let socket = UdpSocket::bind(&local_bind_address)?;

    // Connect to relay server
    let relay_address = format!("{}:{}", root_opts.relay_host, root_opts.relay_port);
    debug!("Connecting to relay-server: {}...", relay_address);
    socket.connect(relay_address)?;

    // Get the hostname of the sender
    let sender_host = hide_or_get_hostname(send_opts.hide_hostname)?;
    debug!("Sender hostname: {}", sender_host);

    // create a hash of the file
    let file_hash = if send_opts.skip_hash {
        AnonymousString(None)
    } else {
        debug!("Creating hash of file...");
        AnonymousString(Some(hash_file_and_seek(&mut file)?))
    };
    debug!("File hash: {}", file_hash);

    serialize_and_send(&socket, "S2X_RP", &S2XRequestPassphraseMessage {
        sender_host,
        file_size,
        file_hash,
        file_name: file_name.to_string(),
    })?;

    let send_ack: X2SPassphraseProvidedMessage = receive_and_parse_and_expect(
        &socket,
        "X2S_PPM",
    )?;

    // Print the passphrase to the user
    println!(
        "{} Passphrase: {}",
        style("[✔]").bold().green(),
        style(&send_ack.passphrase).cyan()
    );

    debug!("Waiting for connection request...");
    let conn_req: X2SSenderConnectToReceiverMessage = receive_and_parse_and_expect(
        &socket,
        "X2S_SCON",
    )?;

    println!(
        "{} Connecting to peer {} ({})...",
        style("[~]").bold().yellow(),
        style(&conn_req.receiver_host).cyan(),
        style(&conn_req.receiver_addr).dim()
    );
    socket.connect(conn_req.receiver_addr)?;

    debug!("Initializing socket connection...");
    init_socket(&socket)?;
    debug!("Ready to send data!");

    // wrap the socket in a "reliable udp socket"
    let mut safe_connection = ReliableUdpSocket::new(socket);

    println!(
        "{} Sending {} bytes (chunk-size: {})...",
        style("[~]").bold().yellow(),
        file_size,
        style(format_size(send_opts.chunk_size, DECIMAL)).dim()
    );

    let progress_bar = new_downloader_progressbar(file_size);

    // Used for calculating the total time taken
    let start_time = current_unix_millis();

    // Used for updating the progressbar
    let mut bytes_sent: u64 = 0;

    // update progress every 25 KiB
    let update_progress_rate = (1024 * 25) / send_opts.chunk_size;
    let mut current_progress = 0;

    let mut buffer: Vec<u8> = vec![0; send_opts.chunk_size as usize];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            progress_bar.finish_with_message("Transfer complete! 🎉");
            safe_connection.end();
            break;
        }

        // Send the data from the buffer over the connection
        safe_connection.write_and_flush(
            &buffer[..bytes_read],
            false,
            send_opts.delay,
        )?;

        bytes_sent += bytes_read as u64;

        current_progress += 1;
        if current_progress % update_progress_rate == 0 {
            progress_bar.set_position(bytes_sent);
        }
    }

    println!(
        "{} File sent successfully in {}s!",
        style("[✔]").bold().green(),
        (current_unix_millis() - start_time) as f64 / 1000.0
    );
    Ok(())
}