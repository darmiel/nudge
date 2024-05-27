use std::fs::File;
use std::io::{Read, Seek};
use std::net::{Ipv4Addr, UdpSocket};

use clap::Parser;
use console::style;
use humansize::{DECIMAL, format_size};

use crate::commands::RootOpts;
use crate::error::Result;
use crate::models::X2SPassphraseProvidedMessage;
use crate::models::S2XRequestPassphraseMessage;
use crate::models::X2SSenderConnectToReceiverMessage;
use crate::utils::reliable_udp::ReliableUdpSocket;
use crate::utils::AnonymousString;
use crate::utils::current_unix_millis;
use crate::utils::hash_file_and_seek;
use crate::utils::hide_or_get_hostname;
use crate::utils::new_downloader_progressbar;
use crate::utils::DEFAULT_CHUNK_SIZE;
use crate::utils::serialize::{receive_and_parse_and_expect, serialize_and_send};
use crate::utils::socket::init_socket;

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

pub fn run(root_opts: &RootOpts, send_opts: &SendOpts) -> Result<()> {
    // check if the file exists and open it
    let mut file = File::open(&send_opts.file)?;
    let file_name = send_opts.file.split('/').last().unwrap_or_default();
    let file_size = file.metadata()?.len();

    let socket = bind_socket()?;
    connect_to_relay_server(&socket, root_opts)?;

    // Get the hostname of the sender
    let sender_host = hide_or_get_hostname(send_opts.hide_hostname)?;
    debug!("Sender hostname: {}", sender_host);

    let file_hash = compute_file_hash(send_opts.skip_hash, &mut file)?;
    debug!("File hash: {}", file_hash);

    // Request a passphrase from the relay-server
    serialize_and_send(&socket, "S2X_RP", &S2XRequestPassphraseMessage {
        sender_host,
        file_size,
        file_hash,
        file_name: file_name.to_string(),
    })?;

    // (Hopefully) receive the passphrase from the relay-server
    let passphrase_message: X2SPassphraseProvidedMessage = receive_and_parse_and_expect(
        &socket,
        "X2S_PPM",
    )?;

    println!(
        "{} Passphrase: {}",
        style("[✔]").bold().green(),
        style(&passphrase_message.passphrase).cyan()
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

    send_file(&socket, &mut file, send_opts, file_size)?;
    Ok(())
}

/// Binds a UDP socket to a local address
///
/// # Errors
///
/// Returns `NudgeError::Io` if binding fails
fn bind_socket() -> Result<UdpSocket> {
    let local_bind_address = (Ipv4Addr::from(0u32), 0);
    debug!("Binding UDP socket to local address: {:?}", local_bind_address);
    Ok(UdpSocket::bind(&local_bind_address)?)
}

/// Connects the UDP socket to the relay server
///
/// # Arguments
///
/// * `socket` - The UDP socket
/// * `root_opts` - Root options containing relay host and port
///
/// # Errors
///
/// Returns `NudgeError::Io` if connection fails
fn connect_to_relay_server(socket: &UdpSocket, root_opts: &RootOpts) -> Result<()> {
    let relay_address = format!("{}:{}", root_opts.relay_host, root_opts.relay_port);
    debug!("Connecting to relay-server: {}...", relay_address);
    Ok(socket.connect(&relay_address)?)
}

/// Computes the hash of the file if not skipped
///
/// # Arguments
///
/// * `skip_hash` - Boolean flag to skip hashing
/// * `file` - Mutable reference to the file to be hashed
///
/// # Errors
///
/// Returns `NudgeError::Io` if hashing or seeking fails
fn compute_file_hash(skip_hash: bool, file: &mut File) -> Result<AnonymousString> {
    if skip_hash {
        Ok(AnonymousString(None))
    } else {
        debug!("Creating hash of file...");
        let hash = hash_file_and_seek(file)?;
        file.seek(std::io::SeekFrom::Start(0))?;
        Ok(AnonymousString(Some(hash)))
    }
}


/// Sends the file to the peer in chunks
///
/// # Arguments
///
/// * `socket` - The UDP socket
/// * `file` - Mutable reference to the file to be sent
/// * `send_opts` - Send options containing delay, chunk size, etc.
/// * `file_size` - Size of the file to be sent
///
/// # Errors
///
/// Returns `NudgeError` if any step of the sending process fails
fn send_file(socket: &UdpSocket, file: &mut File, send_opts: &SendOpts, file_size: u64) -> Result<()> {
    let mut safe_connection = ReliableUdpSocket::new(socket.try_clone()?);
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