use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{Read, Seek };
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use console::style;
use dialoguer::theme::ColorfulTheme;
use gethostname::gethostname;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use crate::error::{NudgeError, Result};

pub mod passphrase;
pub mod reliable_udp;

pub const DEFAULT_RELAY_HOST: &'static str = "127.0.0.1";
pub const DEFAULT_RELAY_PORT: &'static str = "4000";
pub const DEFAULT_CHUNK_SIZE: &'static str = "4096";

#[derive(Serialize, Deserialize, Debug, Ord, PartialEq, PartialOrd, Eq)]
pub struct AnonymousString(pub Option<String>);

impl Display for AnonymousString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(hostname) => write!(f, "{}", hostname),
            None => write!(f, "<anonymous>")
        }
    }
}

/// Function to get the current time in milliseconds since the Unix epoch
pub fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn init_socket(socket: &UdpSocket) -> Result<()> {
    // Set socket read and write timeouts
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    socket.set_write_timeout(Some(Duration::from_secs(1)))?;

    // Synchronize to the next 500ms boundary
    thread::sleep(Duration::from_millis(500 - (current_unix_millis() % 500)));

    // Send packets to establish the connection
    for _ in 0..40 {
        let start_time = current_unix_millis();
        let _ = socket.send(&[0]);
        thread::sleep(Duration::from_millis((50 - (current_unix_millis() - start_time))
            .max(0)));
    }

    // Wait for the connection to be established
    let mut receive_result = Ok(1);
    while receive_result.is_ok() && receive_result.unwrap() == 1 {
        receive_result = socket.recv(&mut [0, 0]);
    }
    socket.send(&[0, 0])?;
    socket.send(&[0, 0])?;

    receive_result = Ok(1);
    while receive_result.is_ok() && receive_result.unwrap() != 2 {
        receive_result = socket.recv(&mut [0, 0]);
    }
    receive_result = Ok(1);
    while receive_result.is_ok() && receive_result.unwrap() == 2 {
        receive_result = socket.recv(&mut [0, 0]);
    }

    Ok(())
}

pub fn serialize_and_send(connection: &UdpSocket, prefix: &str, data: &impl Serialize) -> std::result::Result<(), NudgeError> {
    let serialized_data = serde_json::to_string(data)?;
    let message = format!("{} {}", prefix, serialized_data);
    connection.send(message.as_bytes())?;
    Ok(())
}

pub fn receive_and_parse_and_expect<T: DeserializeOwned>(connection: &UdpSocket, expected_prefix: &str) -> std::result::Result<T, NudgeError> {
    let mut buffer = [0u8; 1024];
    connection.recv(&mut buffer)?;

    let mut buffer_vec = buffer.to_vec();
    buffer_vec.retain(|&byte| byte != 0);

    let message = String::from_utf8_lossy(&buffer_vec);
    if message.starts_with("ERROR ") {
        return Err(NudgeError::ServerError(message.to_string()));
    }

    let prefix = message.split_whitespace().next().unwrap();
    if prefix != expected_prefix {
        return Err(NudgeError::ReceiveExpectationNotMet(
            expected_prefix.to_string(),
            prefix.to_string(),
        ));
    }

    // remove leading/trailing whitespace
    let part = message[expected_prefix.len()..].trim();
    Ok(serde_json::from_str(part)?)
}

pub fn get_hostname() -> Result<String> {
    match gethostname().into_string() {
        Ok(hostname) => Ok(hostname),
        Err(_) => Err(NudgeError::HostnameError),
    }
}

pub fn hide_or_get_hostname(hide: bool) -> Result<AnonymousString> {
    Ok(AnonymousString(if hide { None } else { Some(get_hostname()?) }))
}

pub fn hash_file_and_seek(file: &mut File) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    file.seek(std::io::SeekFrom::Start(0))?;
    Ok(hasher.finalize().to_hex().to_string())
}

pub fn question_theme() -> ColorfulTheme {
    let mut colorful = ColorfulTheme::default();
    colorful.prompt_prefix = style("[?]".to_string()).for_stderr().dim();
    colorful.success_prefix = style("[✔]".to_string()).for_stderr().bold().green();
    colorful.error_prefix = style("[✗]".to_string()).for_stderr().bold().red();
    colorful
}

pub fn new_downloader_progressbar(len: u64) -> ProgressBar {
    let progress_bar = ProgressBar::new(len)
        .with_prefix("[>]");
    progress_bar.set_style(ProgressStyle::with_template("{prefix:.orange} {elapsed_precise} :: |{wide_bar:.white/dim}| :: {bytes}/{total_bytes}")
        .unwrap()
        .progress_chars("█ :"));
    progress_bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_unix_millis() {
        let before = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let millis = current_unix_millis();
        let after = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        assert!(millis >= before && millis <= after, "The current_unix_millis function should return the correct time in milliseconds.");
    }
}
