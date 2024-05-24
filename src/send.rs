use std::fs::File;
use std::io::Read;
use std::net::{Ipv4Addr, UdpSocket};

use clap::Parser;
use console::style;
use indicatif::ProgressBar;

use crate::{models, utils};
use crate::error::NudgeError;
use crate::models::{FileSendAckPayload, SenderConnectToReceiverPayload};
use crate::reliable_udp::ReliableUdpSocket;
use crate::utils::{DEFAULT_BITRATE, DEFAULT_RELAY_HOST, DEFAULT_RELAY_PORT, receive_and_parse_and_expect, serialize_and_send};
use crate::utils::current_unix_millis;

#[derive(Parser, Debug)]
pub struct Send {
    file: String,

    #[clap(short = 'x', long, default_value = DEFAULT_RELAY_HOST)]
    relay_host: String,

    #[clap(short = 'y', long, default_value = DEFAULT_RELAY_PORT)]
    relay_port: u16,

    #[clap(short, long, default_value = "500")]
    delay: u64,

    #[clap(short, long, default_value = DEFAULT_BITRATE)]
    bitrate: u32,
}

impl Send {

    pub fn run(&self) -> Result<(), NudgeError> {
        // check if the file exists and open it
        let mut file = File::open(&self.file)?;
        let file_name = &self.file.split('/').last().unwrap();
        let file_length = file.metadata()?.len();

        // Connect to relay server
        let relay_address = format!("{}:{}", self.relay_host, self.relay_port);
        println!(
            "{} Connecting to relay-server: {}...",
            style("[2/4]").bold().dim(),
            relay_address
        );
        let local_bind_address = (Ipv4Addr::from(0u32), 0);
        let local_socket = UdpSocket::bind(&local_bind_address)?;
        local_socket.connect(relay_address)?;

        // SEND_REQ
        serialize_and_send(&local_socket, "SEND_REQ", &models::FileSendRequestPayload {
            sender_host: "daniel".to_string(), // TODO: change me
            file_size: file_length,
            file_hash: "hash".to_string(), // TODO: change me, let the user disable the hash
            file_name: file_name.to_string(),
        })?;

        // SEND_ACK
        let send_ack: FileSendAckPayload = receive_and_parse_and_expect(
            &local_socket,
            "SEND_ACK",
        )?;

        println!("Received Passphrase: {}", send_ack.passphrase);
        println!("Waiting for connection request...");

        let conn_req: SenderConnectToReceiverPayload = receive_and_parse_and_expect(
            &local_socket,
            "CONNECT",
        )?;

        println!("Connecting to: {}", conn_req.receiver_addr);
        local_socket.connect(conn_req.receiver_addr)?;

        println!("Initializing socket connection...");
        utils::init_socket(&local_socket)?;

        println!("Ready to send data!");

        // Create a buffer with size based on the bitrate
        let buffer: Vec<u8> = vec![0; self.bitrate as usize];
        let mut buffer = buffer.leak();

        // Create a SafeReadWrite wrapper for the connection
        let mut safe_connection = ReliableUdpSocket::new(local_socket);
        let mut bytes_sent: u64 = 0;


        println!(
            "{} Sending file size ({} bytes) to peer...",
            style("[3/4]").bold().dim(),
            file_length
        );
        safe_connection.write_and_flush(&file_length.to_be_bytes(), false, 3000)?;

        println!(
            "{} Sending {} bytes to peer in {} byte-chunks...",
            style("[4/4]").bold().dim(),
            file_length,
            self.bitrate
        );

        let progress_bar = ProgressBar::new(file_length);

        // Used for calculating the total time taken
        let start_time = current_unix_millis();

        // update progress every 25 KiB
        let update_progress_rate = (1024 * 25) / self.bitrate;
        let mut current_progress = 0;

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                progress_bar.finish_with_message("Transfer complete! 🎉");
                safe_connection.end();
                break;
            }

            // Send the data from the buffer over the connection
            safe_connection.write_and_flush(&buffer[..bytes_read], false, self.delay)?;
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
}