use std::borrow::Cow;
use std::fs::OpenOptions;
use std::io::{Write};
use std::net::{Ipv4Addr, UdpSocket};

use clap::Parser;
use console::style;
use dialoguer::Confirm;
use dialoguer::theme::ColorfulTheme;
use indicatif::ProgressBar;

use crate::error::NudgeError;
use crate::models;
use crate::passphrase::Passphrase;
use crate::reliable_udp::ReliableUdpSocket;
use crate::utils::{current_unix_millis, init_socket, receive_and_parse_and_expect, serialize_and_send};
use crate::utils::{DEFAULT_RELAY_HOST, DEFAULT_RELAY_PORT, DEFAULT_BITRATE};

#[derive(Parser, Debug)]
pub struct Get {
    passphrase: String,

    #[clap(short = 'o', long)]
    out_file: String,

    #[clap(short = 'x', long, default_value = DEFAULT_RELAY_HOST)]
    relay_host: String,

    #[clap(short = 'y', long, default_value = DEFAULT_RELAY_PORT)]
    relay_port: u16,

    #[clap(short, long, default_value = "500")]
    delay: u64,

    #[clap(short, long, default_value = DEFAULT_BITRATE)]
    bitrate: u32,

    #[clap(short, long, default_value = "false")]
    force: bool,
}

impl Get {
    pub fn run(&self) -> Result<(), NudgeError> {
        // Opening the file for writing, creating it if it doesn't exist
        let mut file = OpenOptions::new()
            .truncate(false)
            .write(true)
            .create(true)
            .open(&self.out_file)?;

        // Connect to the relay server
        let relay_address = format!("{}:{}", self.relay_host, self.relay_port);
        println!(
            "{} Connecting to relay-server: {}...",
            style("[1/4]").bold().dim(),
            relay_address
        );
        let local_bind_address = (Ipv4Addr::from(0u32), 0);
        let local_socket = UdpSocket::bind(local_bind_address)?;
        local_socket.connect(relay_address)?;

        // RECV_REQ
        let passphrase = Passphrase(Cow::Owned(self.passphrase.clone()));
        serialize_and_send(&local_socket, "RECV_REQ", &models::FileReceiveRequestPayload {
            passphrase: passphrase.clone(),
        })?;

        let recv_ack: models::FileInfo = receive_and_parse_and_expect(&local_socket, "RECV_ACK")?;

        println!(
            "{} {} by {} [{}]",
            style("(INFO)").bold().green(),
            style(&recv_ack.file_name).yellow(),
            style(&recv_ack.sender_host).cyan(),
            humansize::format_size(recv_ack.file_size, humansize::DECIMAL)
        );

        // ask if we really want to download the file
        if !self.force && !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to download the file?")
            .interact()
            .unwrap()
        {
            println!("Nevermind then :(");
            return Ok(());
        }

        println!("Requesting sender to connect to us...");

        // ask the sender to connect to us
        serialize_and_send(&local_socket, "RECV_ACC", &models::FileReceiveAcceptPayload {
            passphrase,
            file_hash: recv_ack.file_hash,
        })?;

        println!("Connecting to: {}", recv_ack.sender_addr);
        local_socket.connect(recv_ack.sender_addr)?;

        println!("Initializing socket connection...");
        init_socket(&local_socket)?;

        println!("Ready to receive data!");

        // Initializing a buffer with size equal to the bitrate
        let buffer: Vec<u8> = vec![0; self.bitrate as usize];
        let buffer: &[u8] = buffer.leak();

        // Wrapping the connection with SafeReadWrite for safe reading and writing
        let mut safe_connection = ReliableUdpSocket::new(local_socket);

        // Reading the length of the data from the sender
        println!(
            "{} Reading file size...",
            style("[2/3]").bold().dim()
        );
        let mut length_buffer = [0u8; 8];
        let length_bytes = safe_connection.read(&mut length_buffer)?.0;
        let data_length = u64::from_be_bytes([
            length_bytes[0], length_bytes[1], length_bytes[2], length_bytes[3],
            length_bytes[4], length_bytes[5], length_bytes[6], length_bytes[7]
        ]);
        file.set_len(data_length)?;

        println!(
            "{} Receiving {} bytes from peer in {} byte-chunks...",
            style("[3/3]").bold().dim(),
            data_length,
            self.bitrate
        );

        let progress_bar = ProgressBar::new(data_length);

        // Used for calculating the total time taken
        let start_time = current_unix_millis();

        // Used for updating the progressbar
        let mut bytes_received: u64 = 0;

        // update progress every 25 KiB
        let update_progress_rate = (1024 * 25) / self.bitrate;
        let mut current_progress = 0;

        loop {
            let (read_buffer, bytes_read) = safe_connection.read(buffer)?;
            if bytes_read == 0 {
                progress_bar.finish_with_message("Transfer complete! 🎉");
                break;
            }

            let buffer = &read_buffer.leak()[..bytes_read];
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
        Ok(())
    }
}