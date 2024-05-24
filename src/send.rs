use std::fs::File;
use std::io::Read;

use clap::Parser;
use console::style;
use indicatif::ProgressBar;

use crate::error::NudgeError;
use crate::passphrase::PassphraseGenerator;
use crate::reliable_udp::ReliableUdpSocket;
use crate::utils::{current_unix_millis, perform_hole_punching};
use crate::utils::{DEFAULT_BITRATE, DEFAULT_RELAY_HOST, DEFAULT_RELAY_PORT};

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

        // generate a random passphrase
        let passphrase_generator = PassphraseGenerator::new()?;
        let passphrase = passphrase_generator.generate()
            .expect("Could not generate passphrase");

        println!(
            "{} Generated passphrase: {}",
            style("[1/4]").bold().dim(),
            passphrase
        );

        let relay_address = format!("{}:{}", self.relay_host, self.relay_port);
        println!(
            "{} Connecting to relay-server at {} and performing hole punch...",
            style("[2/4]").bold().dim(),
            relay_address
        );
        let connection = perform_hole_punching(relay_address, passphrase.to_string())?;

        // Create a buffer with size based on the bitrate
        let buffer: Vec<u8> = vec![0; self.bitrate as usize];
        let mut buffer = buffer.leak();

        // Create a SafeReadWrite wrapper for the connection
        let mut safe_connection = ReliableUdpSocket::new(connection);
        let mut bytes_sent: u64 = 0;

        let file_length = file.metadata()?.len();
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