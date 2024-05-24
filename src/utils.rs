use std::net::UdpSocket;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{NudgeError, Result};

pub const DEFAULT_RELAY_HOST: &'static str = "127.0.0.1";
pub const DEFAULT_RELAY_PORT: &'static str = "4000";
pub const DEFAULT_BITRATE: &'static str = "4096";

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
