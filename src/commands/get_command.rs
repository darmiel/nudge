use std::borrow::Cow;
use std::fs::OpenOptions;
use std::io::{Seek, Write};
use std::net::{Ipv4Addr, UdpSocket};
use std::path::Path;

use clap::Parser;
use console::style;
use dialoguer::Confirm;
use humansize::{DECIMAL, format_size};
use crate::commands::RootOpts;

use crate::error::NudgeError;
use crate::models::FileInfo;
use crate::models::R2XRequestSenderConnectionMessage;
use crate::models::R2XRequestFileInfoMessage;
use crate::utils::passphrase::Passphrase;
use crate::utils::reliable_udp::ReliableUdpSocket;
use crate::utils::{current_unix_millis, hash_file_and_seek};
use crate::utils::hide_or_get_hostname;
use crate::utils::init_socket;
use crate::utils::new_downloader_progressbar;
use crate::utils::question_theme;
use crate::utils::receive_and_parse_and_expect;
use crate::utils::serialize_and_send;
use crate::utils::DEFAULT_CHUNK_SIZE;

#[derive(Parser, Debug)]
pub struct GetOpts {
    /// Passphrase to access the file (required)
    passphrase: String,

    /// Override the output file (optional)
    #[clap(short = 'o', long)]
    out_file: Option<String>,

    #[clap(short, long, default_value = "500")]
    delay: u64,

    /// If enabled, won't ask for confirmation before downloading the file
    #[clap(short, long, default_value = "false")]
    force: bool,

    /// If enabled, won't send the hostname to the sender
    #[clap(long, default_value = "false")]
    hide_hostname: bool,

    /// If enabled, will overwrite the output file if it already exists without asking
    #[clap(long, default_value = "false")]
    overwrite_file: bool,

    /// If enabled, won't display any prompts and always quit
    /// Useful for scripting
    #[clap(long, default_value = "false")]
    no_prompt: bool,

    /// If enabled, won't check the hash of the file
    #[clap(long, default_value = "false")]
    skip_hash: bool,

    /// Chunk size to read from the socket
    #[clap(short, long, default_value = DEFAULT_CHUNK_SIZE)]
    chunk_size: u32,
}

pub fn run(root_opts: &RootOpts, get_opts: &GetOpts) -> Result<(), NudgeError> {
    let local_bind_address = (Ipv4Addr::from(0u32), 0);
    debug!("Binding UDP socket to local address: {:?}", local_bind_address);
    let socket = UdpSocket::bind(local_bind_address)?;

    let relay_address = format!("{}:{}", root_opts.relay_host, root_opts.relay_port);
    debug!("Connecting to relay-server: {}...", relay_address);
    socket.connect(relay_address)?;

    // RECV_REQ
    let passphrase = Passphrase(Cow::Owned(get_opts.passphrase.clone()));
    debug!("Sending R2XRequestFileInfoMessage with passphrase: {}...", passphrase.0);
    serialize_and_send(&socket, "R2X_RFI", &R2XRequestFileInfoMessage {
        passphrase: passphrase.clone(),
    })?;

    debug!("Waiting for FileInfo...");
    let recv_ack: FileInfo = receive_and_parse_and_expect(&socket, "X2R_AFI")?;
    debug!("Received FileInfo: {:?}", recv_ack);

    // display file information
    println!(
        "{} Meta: {} by {} [{}]",
        style("[✔]").bold().green(),
        style(&recv_ack.file_name).yellow(),
        style(&recv_ack.sender_host).cyan(),
        format_size(recv_ack.file_size, DECIMAL)
    );

    let out_file_name = if let Some(out_file) = &get_opts.out_file {
        out_file.as_str()
    } else {
        // if the output file is not specified, use the file name from the sender
        // this is the last part of the file path to prevent directory traversal
        recv_ack.file_name.split("/").last().expect("File name is empty")
    };

    // if the file already exists, ask if we want to overwrite it
    if !get_opts.overwrite_file && Path::new(out_file_name).exists() {
        if get_opts.no_prompt {
            println!("File {} already exists. Use -o <file> to specify a different output file.", out_file_name);
            return Err(NudgeError::NoPromptExit);
        }

        // ask for confirmation to overwrite the file
        if !Confirm::with_theme(&question_theme())
            .with_prompt(format!("File {} already exists. Overwrite?", out_file_name))
            .interact()
            .unwrap()
        {
            println!("Cancelled by user. You can specify a different output file with -o <file>.");
            return Ok(());
        }
    }

    // ask if we really want to download the file
    if !get_opts.force {
        // never download if not -f and --no-prompt passed
        if get_opts.no_prompt {
            println!("Do you want to download the file? Pass -f to download without asking.");
            return Err(NudgeError::NoPromptExit);
        }

        // ask for confirmation
        if !Confirm::with_theme(&question_theme())
            .with_prompt("Do you want to download the file?")
            .interact()
            .unwrap()
        {
            println!("Cancelled by user.");
            return Ok(());
        }
    }

    // Opening the file for writing, creating it if it doesn't exist
    let mut file = OpenOptions::new()
        .truncate(false)
        .write(true)
        .create(true)
        .read(true)
        .open(out_file_name)?;
    file.set_len(recv_ack.file_size)?;

    // ask the sender to connect to us
    let hostname = hide_or_get_hostname(get_opts.hide_hostname)?;
    debug!(
            "Requesting sender to connect to us ({}) via R2XRequestSenderConnectionMessage...",
            hostname
        );
    serialize_and_send(&socket, "R2X_RSC", &R2XRequestSenderConnectionMessage {
        passphrase,
        file_hash: recv_ack.file_hash.clone(),
        receiver_host: hostname,
    })?;

    println!(
        "{} Connecting to {} ({})...",
        style("[~]").bold().yellow(),
        style(&recv_ack.sender_host).cyan(),
        style(&recv_ack.sender_addr).dim()
    );
    socket.connect(recv_ack.sender_addr)?;

    debug!("Initializing socket connection...");
    init_socket(&socket)?;
    debug!("Ready to receive data!");

    // wrap the socket in a "reliable udp socket"
    let mut safe_connection = ReliableUdpSocket::new(socket);

    println!(
        "{} Receiving {} (chunk-size: {})...",
        style("[~]").bold().yellow(),
        format_size(recv_ack.file_size, DECIMAL),
        style(format_size(get_opts.chunk_size, DECIMAL)).dim()
    );

    let progress_bar = new_downloader_progressbar(recv_ack.file_size);

    // Used for calculating the total time taken
    let start_time = current_unix_millis();

    // Used for updating the progressbar
    let mut bytes_received: u64 = 0;

    // update progress every 25 KiB
    let update_progress_rate = (1024 * 25) / get_opts.chunk_size;
    let mut current_progress = 0;

    let mut buffer: Vec<u8> = vec![0; get_opts.chunk_size as usize];

    loop {
        let (read_buffer, bytes_read) = safe_connection.read(&mut buffer)?;
        if bytes_read == 0 {
            progress_bar.finish_with_message("Transfer complete! 🎉");
            break;
        }

        let buffer = &read_buffer[..bytes_read];
        file.write_all(buffer)?;
        file.flush()?;

        bytes_received += bytes_read as u64;

        current_progress += 1;
        if current_progress % update_progress_rate == 0 {
            progress_bar.set_position(bytes_received);
        }
    }

    println!(
        "{} File received successfully in {}s!",
        style("[✔]").bold().green(),
        (current_unix_millis() - start_time) as f64 / 1000.0
    );

    if get_opts.skip_hash {
        // if the hash is skipped, we don't need to check it
        return Ok(());
    }

    // if no hash was sent, display warning to the user
    // we only treat this case as a warning, not an error
    if recv_ack.file_hash.0.is_none() {
        println!(
            "{} Sender did not send a hash! Skipping hash check...",
            style("[✗]").bold().red()
        );
        return Ok(());
    }

    let expected_hash = recv_ack.file_hash.0.unwrap();
    println!(
        "{} Checking file hash... Expected: {}",
        style("[~]").bold().yellow(),
        style(&expected_hash).dim()
    );

    file.seek(std::io::SeekFrom::Start(0))?;
    let actual_hash = hash_file_and_seek(&mut file)?;

    if expected_hash != actual_hash {
        println!(
            "{} Hash mismatch! Expected: {}, Received: {}",
            style("[✗]").bold().red(),
            expected_hash,
            actual_hash
        );
        return Err(NudgeError::HashMismatch(expected_hash, actual_hash));
    }

    println!(
        "{} Hash check successful! ({})",
        style("[✔]").bold().green(),
        expected_hash
    );

    Ok(())
}