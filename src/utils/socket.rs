use std::net::UdpSocket;
use std::time::Duration;
use std::thread;

use crate::error::Result;
use crate::utils::current_unix_millis;

/// Synchronizes the thread to the next boundary of the specified interval in milliseconds.
///
/// # Arguments
///
/// * `interval_ms` - The interval in milliseconds to synchronize to.
fn sync_to_next_boundary(interval_ms: u64) {
    let delay = interval_ms - (current_unix_millis() % interval_ms);
    thread::sleep(Duration::from_millis(delay));
}

/// Sends a specified number of packets at the given interval in milliseconds.
///
/// # Arguments
///
/// * `socket` - A reference to the `UdpSocket`.
/// * `count` - The number of packets to send.
/// * `interval_ms` - The interval in milliseconds between each packet.
///
/// # Returns
///
/// * `Result<()>` - An `Ok` result if all packets are sent successfully, or an error otherwise.
fn send_packets(socket: &UdpSocket, count: usize, interval_ms: u64) -> Result<()> {
    for _ in 0..count {
        let start_time = current_unix_millis();
        let _ = socket.send(&[0]);
        let elapsed = current_unix_millis() - start_time;
        thread::sleep(Duration::from_millis((interval_ms as i64 - elapsed as i64).max(0) as u64));
    }
    Ok(())
}

/// Waits for a condition to be met based on the data received from the socket.
///
/// # Arguments
///
/// * `socket` - A reference to the `UdpSocket`.
/// * `condition` - A closure that takes the received byte count and returns a boolean indicating whether to continue waiting.
///
/// # Returns
///
/// * `Result<()>` - An `Ok` result when the condition is met, or an error if a socket operation fails.
fn wait_for_condition<F>(socket: &UdpSocket, condition: F) -> Result<()>
    where
        F: Fn(usize) -> bool,
{
    let mut buffer = [0; 2];
    while let Ok(received) = socket.recv(&mut buffer) {
        if !condition(received) {
            break;
        }
    }
    Ok(())
}

/// Initializes a UDP socket by setting timeouts, synchronizing to a boundary,
/// sending initial packets, and waiting for a connection to be established.
///
/// # Arguments
///
/// * `socket` - A reference to the `UdpSocket`.
///
/// # Returns
///
/// * `Result<()>` - An `Ok` result if the initialization succeeds, or an error otherwise.
pub fn init_socket(socket: &UdpSocket) -> Result<()> {
    // Set socket read and write timeouts
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    socket.set_write_timeout(Some(Duration::from_secs(1)))?;

    // Synchronize to the next 500ms boundary
    sync_to_next_boundary(500);

    // Send packets to establish the connection
    send_packets(socket, 40, 50)?;

    // Wait for the connection to be established
    wait_for_condition(socket, |received| received == 1)?;
    socket.send(&[0, 0])?;
    socket.send(&[0, 0])?;

    wait_for_condition(socket, |received| received != 2)?;
    wait_for_condition(socket, |received| received == 2)?;

    Ok(())
}