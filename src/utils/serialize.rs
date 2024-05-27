use serde::{Serialize};
use serde::de::DeserializeOwned;
use std::net::UdpSocket;

use crate::error::{NudgeError, Result};

/// Serializes the given data and sends it over the provided UDP socket with the specified prefix.
///
/// # Arguments
///
/// * `connection` - A reference to the `UdpSocket` used for sending the data.
/// * `prefix` - A string slice that will be prefixed to the serialized data.
/// * `data` - A reference to the data to be serialized and sent.
///
/// # Errors
///
/// Returns `NudgeError` if serialization fails or if sending the message fails.
pub fn serialize_and_send(socket: &UdpSocket, prefix: &str, data: &impl Serialize) -> Result<()> {
    let serialized_data = serde_json::to_string(data)?;
    let message = format!("{} {}", prefix, serialized_data);
    socket.send(message.as_bytes())?;
    Ok(())
}

/// Receives a message from the UDP socket, parses it, and checks if it matches the expected prefix.
///
/// # Arguments
///
/// * `connection` - A reference to the `UdpSocket` used for receiving the data.
/// * `expected_prefix` - The expected prefix of the received message.
///
/// # Errors
///
/// Returns `NudgeError` if receiving the message fails, if the message contains an error,
/// if the prefix does not match, or if deserialization fails.
pub fn receive_and_parse_and_expect<T>(connection: &UdpSocket, expected_prefix: &str) -> Result<T>
    where
        T: DeserializeOwned
{
    let mut buffer = [0u8; 1024];
    connection.recv(&mut buffer)?;

    let message = String::from_utf8_lossy(&buffer);
    let message_trimmed = message.trim_end_matches(char::from(0));

    if message_trimmed.starts_with("ERROR ") {
        return Err(NudgeError::ServerError(message_trimmed.to_string()));
    }

    let prefix = message_trimmed.split_whitespace().next().unwrap_or_default();
    if prefix != expected_prefix {
        return Err(NudgeError::ReceiveExpectationNotMet(
            expected_prefix.to_string(),
            prefix.to_string(),
        ));
    }

    let part = message_trimmed[prefix.len()..].trim();
    Ok(serde_json::from_str(part)?)
}
