use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::thread;
use std::time::SystemTime;

use std::time::Duration;

use crate::error::Result;

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

/// Function to initiate a hole-punching process with the given address and passphrase
pub fn perform_hole_punching(relay_address: String, passphrase: String) -> Result<UdpSocket> {
    // Bind the socket to any available port on the local machine
    let local_bind_address = (Ipv4Addr::from(0u32), 0);
    let socket = UdpSocket::bind(&local_bind_address)?;

    // Connect the socket to the relay's address
    socket.connect(relay_address)?;

    // Prepare the buffer and send the passphrase to the relay
    let mut buffer = [0u8; 200];
    for i in 0..passphrase.len().min(200) {
        buffer[i] = passphrase.as_bytes()[i];
    }
    socket.send(&buffer)?;
    socket.recv(&mut buffer)?;

    // Process the received data to extract the peer's bind address
    let mut address_bytes = Vec::from(buffer);
    address_bytes.retain(|&byte| byte != 0);
    let peer_bind_address_str = String::from_utf8_lossy(&address_bytes).to_string();

    // Convert the peer's bind address string to a SocketAddrV4
    let peer_bind_address: SocketAddrV4 = peer_bind_address_str.parse()
        .expect("Invalid peer address format");

    // Reconnect the socket to the newly parsed peer address
    socket.connect(peer_bind_address)?;

    // Set socket read and write timeouts
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    socket.set_write_timeout(Some(Duration::from_secs(1)))?;

    // Synchronize to the next 500ms boundary
    thread::sleep(Duration::from_millis(500 - (current_unix_millis() % 500)));

    // Send packets to establish the connection
    for _ in 0..40 {
        let start_time = current_unix_millis();
        let _ = socket.send(&[0]);
        thread::sleep(Duration::from_millis((50 - (current_unix_millis() - start_time)).max(0)));
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
    Ok(socket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

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

    #[test]
    fn test_perform_hole_punching_invalid_address() {
        let relay_address = "256.256.256.256:0".to_string(); // invalid IP address
        let passphrase = "test_passphrase".to_string();
        let result = perform_hole_punching(relay_address, passphrase);
        assert!(result.is_err(), "Expected error with invalid relay address.");
    }

    #[test]
    fn test_perform_hole_punching_connection() -> Result<()> {
        let server_socket = UdpSocket::bind("127.0.0.1:0")?;
        let server_address = server_socket.local_addr()?;
        let relay_address = server_address.to_string();

        let peer_address = "127.0.0.1:12345";
        thread::spawn(move || {
            let mut buf = [0; 200];
            let (_, src) = server_socket.recv_from(&mut buf).unwrap();
            server_socket.send_to(peer_address.as_bytes(), src).unwrap();
        });

        let passphrase = "test_passphrase".to_string();
        let socket = perform_hole_punching(relay_address, passphrase)?;

        assert_eq!(socket.peer_addr()?.to_string(), "127.0.0.1:12345", "Socket should be connected to the correct peer address.");
        Ok(())
    }
}
