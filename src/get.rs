use std::fs::OpenOptions;
use std::io::{Write};

use clap::Parser;
use console::style;
use indicatif::ProgressBar;

use crate::error::NudgeError;
use crate::reliable_udp::ReliableUdpSocket;
use crate::utils::{current_unix_millis, perform_hole_punching};
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
}

impl Get {
    pub fn run(&self) -> Result<(), NudgeError> {
        // Opening the file for writing, creating it if it doesn't exist
        let mut file = OpenOptions::new()
            .truncate(false)
            .write(true)
            .create(true)
            .open(&self.out_file)?;

        // Initializing a buffer with size equal to the bitrate
        let buffer: Vec<u8> = vec![0; self.bitrate as usize];
        let buffer: &[u8] = buffer.leak();

        let relay_address = format!("{}:{}", self.relay_host, self.relay_port);
        println!(
            "{} Connecting to relay-server at {} and performing hole punch...",
            style("[1/3]").bold().dim(),
            relay_address
        );
        let connection = perform_hole_punching(relay_address, self.passphrase.clone())?;

        // Wrapping the connection with SafeReadWrite for safe reading and writing
        let mut safe_connection = ReliableUdpSocket::new(connection);


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