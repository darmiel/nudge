use std::collections::HashMap;
use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

use crate::error::{NudgeError, Result};
use crate::utils::current_unix_millis;

#[derive(Ord, Eq, PartialOrd, PartialEq)]
enum PacketType {
    Write,
    Acknowledgment,
    ResendRequest,
    EndSession,
}

/// Handles reliable data transmission over UDP with manual acknowledgments and retransmissions.
/// Heavily inspired by SafeReadWrite from https://github.com/TudbuT/qft/blob/master/src/main.rs
pub struct ReliableUdpSocket {
    socket: UdpSocket,
    last_transmitted: HashMap<u16, Vec<u8>>,
    sent_packets_count: u64,
    received_packets_count: u64,
}

impl ReliableUdpSocket {
    /// Creates a new instance bound to the provided UDP socket.
    pub fn new(socket: UdpSocket) -> Self {
        ReliableUdpSocket {
            socket,
            last_transmitted: HashMap::new(),
            received_packets_count: 0,
            sent_packets_count: 0,
        }
    }

    /// Safely writes data to the socket with an optional flush and delay.
    pub fn write_and_flush(&mut self, data: &[u8], should_flush: bool, delay: u64) -> Result<()> {
        self.internal_write(data, PacketType::Write, should_flush, false, delay)
    }

    /// Reads data from the socket, ensuring packet order and requesting retransmissions if necessary.
    pub fn read(&mut self, buffer: &[u8]) -> Result<(Vec<u8>, usize)> {
        if buffer.len() > 0xfffc {
            return Err(NudgeError::BufferSizeLimitExceeded(buffer.len()));
        }

        let mut packet_buffer = Vec::with_capacity(buffer.len() + 3);
        packet_buffer.extend_from_slice(&[0; 3]); // Prepend three bytes for the packet header
        packet_buffer.extend_from_slice(buffer);

        let mut received_data = (Vec::new(), 0);
        let mut should_retry = true;
        let mut is_catching_up = false;

        while should_retry {
            match self.socket.recv(&mut packet_buffer) {
                Ok(bytes_read) => {
                    if bytes_read < 3 { continue; }
                    let packet_id = u16::from_be_bytes([packet_buffer[0], packet_buffer[1]]);
                    self.handle_packet(packet_id, &mut packet_buffer, &mut received_data, &mut should_retry, &mut is_catching_up, bytes_read)?;
                }
                Err(_) => continue,
            }
        }
        packet_buffer.drain(0..3); // Remove the header
        received_data.0 = packet_buffer;
        Ok(received_data)
    }

    /// Ends the session, ensuring all data is flushed and the socket is properly closed.
    pub fn end(mut self) -> UdpSocket {
        let _ = self.internal_write(&[], PacketType::EndSession, true, true, 3000);
        self.socket
    }

    /// Internal method to handle packet writing with retries and error handling.
    fn internal_write(
        &mut self,
        data: &[u8],
        packet_type: PacketType,
        flush: bool,
        exit_on_lost: bool,
        delay: u64,
    ) -> Result<()> {
        if data.len() > 0xfffc {
            return Err(NudgeError::DataPacketLimitExceeded(data.len()));
        }

        let packet_id = (self.sent_packets_count as u16).to_be_bytes();
        let packet_index = self.sent_packets_count as u16;
        self.sent_packets_count += 1;

        let mut data_buffer = Vec::from(data);
        data_buffer.insert(0, packet_type as u8);
        data_buffer.insert(0, packet_id[1]);
        data_buffer.insert(0, packet_id[0]); // Prepend packet header

        // Transmit the packet with retries if not acknowledged
        self.transmit_packet(&data_buffer, packet_index, delay, flush, exit_on_lost)
    }

    /// Handles received packets, managing acknowledgment responses and detecting packet drops.
    fn handle_packet(&mut self,
                     packet_id: u16,
                     packet_buffer: &mut Vec<u8>,
                     received_data: &mut (Vec<u8>, usize),
                     should_retry: &mut bool,
                     is_catching_up: &mut bool,
                     bytes_read: usize,
    ) -> Result<()> {
        if packet_id <= self.received_packets_count as u16 {
            self.socket.send(&[packet_buffer[0], packet_buffer[1], PacketType::Acknowledgment as u8])?;
        }
        if packet_id == self.received_packets_count as u16 {
            *should_retry = false;
            self.received_packets_count += 1;
            received_data.1 = bytes_read - 3;
        } else if packet_id > self.received_packets_count as u16 {
            self.handle_packet_drop(packet_id, is_catching_up)?;
        }
        if packet_buffer[2] == PacketType::EndSession as u8 {
            *should_retry = false;
        }
        Ok(())
    }

    /// Resends packets from `last_transmitted` map based on received requests or packet loss detection.
    fn transmit_packet(&mut self,
                       data_buffer: &[u8],
                       packet_index: u16,
                       delay: u64,
                       flush: bool,
                       exit_on_lost: bool,
    ) -> Result<()> {
        loop {
            match self.socket.send(data_buffer) {
                Ok(bytes_sent) => {
                    if bytes_sent != data_buffer.len() {
                        continue;
                    }
                }
                Err(_) => continue,
            }
            thread::sleep(Duration::from_micros(delay));
            self.last_transmitted.insert(packet_index, data_buffer.to_vec());
            break;
        }
        self.wait_for_acknowledgment(packet_index, flush, exit_on_lost)
    }

    /// Waits for an acknowledgment for the specified packet. Handles timeouts and retransmissions.
    fn wait_for_acknowledgment(&mut self, packet_index: u16, flush: bool, exit_on_lost: bool) -> Result<()> {
        let mut wait_for_ack = packet_index == 0xffff || flush;
        self.socket.set_read_timeout(Some(Duration::from_millis(1000))).unwrap();

        let mut start_time = current_unix_millis();
        let mut buffer = [0; 3];
        let mut is_catching_up = false;

        while wait_for_ack {
            match self.socket.recv(&mut buffer) {
                Ok(bytes_read) => {
                    if bytes_read != 3 {
                        continue; // Expecting exactly 3 bytes, retry if not received
                    }

                    match buffer[2] {
                        x if x == PacketType::Acknowledgment as u8 => {
                            let acknowledged_packet_id = u16::from_be_bytes([buffer[0], buffer[1]]);
                            self.last_transmitted.remove(&acknowledged_packet_id);
                            if acknowledged_packet_id == packet_index {
                                wait_for_ack = false;
                                self.last_transmitted.clear(); // Clearing as all prior must be ACK'd if ordered.
                            }
                        }
                        x if x == PacketType::ResendRequest as u8 => {
                            let request_packet_id = u16::from_be_bytes([buffer[0], buffer[1]]);
                            self.handle_resend_request(request_packet_id, &mut is_catching_up);
                        }
                        _ => continue, // Ignoring unrecognized packet types
                    }
                }
                Err(_) => {
                    if current_unix_millis() - start_time > 5000 && exit_on_lost {
                        println!("WARN: No acknowledgment received within 5 seconds, potential packet loss");
                        break; // Exit if no response and exiting on loss is specified.
                    }
                    if current_unix_millis() - start_time > 10000 {
                        println!("WARN: Connection may be disrupted. It's been 10 seconds since the last packet was received. Attempting to resend...");
                        if let Some(data) = self.last_transmitted.get(&packet_index).cloned() {
                            self.resend_packet(&data, &mut start_time);
                            start_time = current_unix_millis(); // Reset the timer after resending
                        } else {
                            break; // Exit loop if the latest packet was ACK'd
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handles packet resend requests from the receiver, using the specified packet ID.
    fn handle_resend_request(&mut self, packet_index: u16, is_catching_up: &mut bool) {
        *is_catching_up = true; // Flagging as catching up due to a resend request.

        // Clone the packet data first to avoid borrowing issues
        if let Some(packet_data) = self.last_transmitted.get(&packet_index).cloned() {
            let mut current_time = current_unix_millis(); // Get the current time once, before calling resend_packet
            self.resend_packet(&packet_data, &mut current_time);
        }
    }

    /// Resends a packet and resets the start time for response waiting.
    fn resend_packet(&mut self, packet_data: &[u8], start_time: &mut u64) {
        loop {
            match self.socket.send(packet_data) {
                Ok(bytes_sent) => {
                    if bytes_sent == packet_data.len() {
                        break; // Break if packet is sent successfully
                    }
                }
                Err(_) => continue, // Retry on send error
            }
            thread::sleep(Duration::from_millis(4)); // Minimal delay between retries
        }
        *start_time = current_unix_millis(); // Reset timer after successful resend
    }

    /// Detects and handles the event of packet drop based on the ID discrepancies.
    fn handle_packet_drop(&mut self, packet_id: u16, is_catching_up: &mut bool) -> Result<()> {
        if !*is_catching_up {
            println!(
                "WARN: A packet was dropped: received ID {} is more recent than the expected ID {}",
                packet_id, self.received_packets_count
            );
            *is_catching_up = true;
        }
        // Request resend for the missing packet
        let expected_packet_id = (self.received_packets_count as u16).to_be_bytes();
        self.socket.send(&[expected_packet_id[0], expected_packet_id[1], PacketType::ResendRequest as u8])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::UdpSocket;

    use super::*;

    #[test]
    fn test_reliable_udp_socket_new() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let reliable_socket = ReliableUdpSocket::new(socket.try_clone().unwrap());
        assert_eq!(reliable_socket.sent_packets_count, 0);
        assert_eq!(reliable_socket.received_packets_count, 0);
    }

    #[test]
    fn test_internal_write_data_packet_limit_exceeded() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut reliable_socket = ReliableUdpSocket::new(socket.try_clone().unwrap());

        let large_data = vec![0u8; 0x10000];
        let result = reliable_socket.internal_write(&large_data, PacketType::Write, false, false, 10);
        assert!(matches!(result, Err(NudgeError::DataPacketLimitExceeded(_))));
    }

}
