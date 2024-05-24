use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use clap::Parser;

use crate::error::{NudgeError, Result};
use crate::passphrase::{Passphrase, PassphraseGenerator};
use crate::utils::current_unix_millis;
use crate::models::*;

#[derive(Parser, Debug)]
pub struct RelayServerOpts {
    #[clap(short = 'x', long, default_value = "0.0.0.0")]
    host: String,

    #[clap(short, long, default_value = "4000")]
    port: u16,
}

pub struct RelayServer {
    opts: RelayServerOpts,
    passphrase_generator: PassphraseGenerator,
    client_map: HashMap<Passphrase<'static>, FileInfo>,
}

impl RelayServer {
    pub fn new(opts: RelayServerOpts) -> Result<Self> {
        let passphrase_generator = PassphraseGenerator::new()?;
        let client_map = HashMap::new();

        Ok(Self {
            opts,
            passphrase_generator,
            client_map,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let bind_addr = format!("{}:{}", self.opts.host, self.opts.port);
        println!("Starting server on {}", bind_addr);

        let listener = UdpSocket::bind(&bind_addr)?;

        let mut buf = [0u8; 1024];

        loop {
            let (len, addr) = listener.recv_from(&mut buf)?;
            println!("Received {} bytes from {}", len, addr);

            let received_str = std::str::from_utf8(&buf[..len])?;
            if received_str.starts_with("SEND_REQ ") {
                match self.handle_send_request(&listener, &addr, &received_str[9..]) {
                    Ok(_) => println!("Successfully handled SEND_REQ"),
                    Err(e) => println!("Failed to handle SEND_REQ: {}", e),
                }
                continue;
            }

            if received_str.starts_with("RECV_REQ ") {
                match self.handle_receive_request(&listener, &addr, &received_str[9..]) {
                    Ok(_) => println!("Successfully handled RECEIVE_REQ"),
                    Err(e) => println!("Failed to handle RECEIVE_REQ: {}", e),
                }
                continue;
            }

            // send receiver address to sender
            if received_str.starts_with("RECV_ACC ") {
                match self.handle_receive_accept(&listener, &addr, &received_str[9..]) {
                    Ok(_) => println!("Successfully handled RECEIVE_ACCEPT"),
                    Err(e) => println!("Failed to handle RECEIVE_ACCEPT: {}", e),
                }
                continue;
            }
        }
    }

    /// Handle a SEND_REQ packet
    fn handle_send_request(&mut self, listener: &UdpSocket, addr: &SocketAddr, payload_str: &str) -> Result<()> {
        let payload: FileSendRequestPayload = serde_json::from_str(payload_str)?;

        let file_info = FileInfo {
            file_size: payload.file_size,
            file_name: payload.file_name,
            file_hash: payload.file_hash,
            created_at: current_unix_millis(),
            sender_host: payload.sender_host,
            sender_addr: *addr,
        };

        let passphrase = self.passphrase_generator.generate()
            .ok_or(NudgeError::PassphraseGenerationError)?;

        self.client_map.insert(passphrase.clone(), file_info);
        self.send_send_ack(listener, addr, passphrase)
    }

    /// Send a SEND_ACK packet
    fn send_send_ack(&self, listener: &UdpSocket, addr: &SocketAddr, passphrase: Passphrase<'static>) -> Result<()> {
        let response_payload = FileSendAckPayload { passphrase };
        let response = format!("SEND_ACK {}\n", serde_json::to_string(&response_payload)?);
        listener.send_to(response.as_bytes(), addr)?;

        Ok(())
    }

    fn handle_receive_request(&mut self, listener: &UdpSocket, addr: &SocketAddr, payload_str: &str) -> Result<()> {
        let payload: FileReceiveRequestPayload = serde_json::from_str(payload_str)?;

        if let Some(file_info) = self.client_map.get(&payload.passphrase) {
            self.send_receive_ack(listener, addr, file_info)
        } else {
            Err(NudgeError::PassphraseNotFound)
        }
    }

    fn send_receive_ack(&self, listener: &UdpSocket, addr: &SocketAddr, file_info: &FileInfo) -> Result<()> {
        let response = format!("RECV_ACK {}\n", serde_json::to_string(file_info)?);
        listener.send_to(response.as_bytes(), addr)?;
        Ok(())
    }

    fn handle_receive_accept(&mut self, listener: &UdpSocket, addr: &SocketAddr, payload_str: &str) -> Result<()> {
        let payload: FileReceiveAcceptPayload = serde_json::from_str(payload_str)?;

        if let Some(file_info) = self.client_map.get(&payload.passphrase) {
            // make sure the file hash matches
            if file_info.file_hash == payload.file_hash {
                println!(
                    "File hash matches, sending sender ({}) to receiver ({})",
                    file_info.sender_addr, addr
                );

                // Clone the necessary info before removing the entry
                let sender_addr = file_info.sender_addr.clone();

                self.client_map.remove(&payload.passphrase);

                self.send_sender_connect_to_receiver(listener, &sender_addr, addr)
            } else {
                Err(NudgeError::PassphraseNotFound)
            }
        } else {
            Err(NudgeError::PassphraseNotFound)
        }
    }

    fn send_sender_connect_to_receiver(&self,
                                       listener: &UdpSocket,
                                       sender_addr: &SocketAddr,
                                       receiver_addr: &SocketAddr,
    ) -> Result<()> {
        let response_payload = SenderConnectToReceiverPayload { receiver_addr: *receiver_addr };
        let response = format!("CONNECT {}\n", serde_json::to_string(&response_payload)?);
        listener.send_to(response.as_bytes(), sender_addr)?;

        Ok(())
    }
}
