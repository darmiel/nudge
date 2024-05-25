use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use clap::Parser;
use crate::commands::RootOpts;

use crate::error::{NudgeError, Result};
use crate::utils::passphrase::{Passphrase, PassphraseGenerator};
use crate::utils::{AnonymousString, current_unix_millis};
use crate::models::*;

#[derive(Parser, Debug)]
pub struct RelayServerOpts {
    // #[clap(short = 'x', long, default_value = "0.0.0.0")]
    // host: String,
    //
    // #[clap(short, long, default_value = "4000")]
    // port: u16,
}

pub fn run(root_opts: &RootOpts, _: &RelayServerOpts) -> Result<()> {
    let passphrase_generator = PassphraseGenerator::new()?;
    let mut client_map = HashMap::new();

    let bind_addr = format!("{}:{}", root_opts.relay_host, root_opts.relay_port);
    println!("Starting server on {}", bind_addr);

    let listener = UdpSocket::bind(&bind_addr)?;

    let mut buf = [0u8; 1024];

    loop {
        let (len, addr) = listener.recv_from(&mut buf)?;
        println!("\nReceived {} bytes from {}", len, addr);
        println!("Received: {:?}", std::str::from_utf8(&buf[..len])?);

        let received_str = std::str::from_utf8(&buf[..len])?;

        // Sender -> Server; Request Passphrase
        if received_str.starts_with("S2X_RP ") {
            match handle_sender_request_passphrase_message(&listener, &addr, &received_str[7..], &passphrase_generator, &mut client_map) {
                Ok(_) => println!("Successfully handled S2X_RP"),
                Err(e) => println!("Failed to handle S2X_RP: {}", e),
            }
            continue;
        }

        // Receiver -> Server; Request File Info
        if received_str.starts_with("R2X_RFI ") {
            match handle_receiver_request_file_info(&listener, &addr, &received_str[8..], &client_map) {
                Ok(_) => println!("Successfully handled R2X_RFI"),
                Err(e) => println!("Failed to handle R2X_RFI: {}", e),
            }
            continue;
        }

        // send receiver address to sender
        if received_str.starts_with("R2X_RSC ") {
            match handle_receiver_accept(&listener, &addr, &received_str[8..], &mut client_map) {
                Ok(_) => println!("Successfully handled R2X_RSC"),
                Err(e) => println!("Failed to handle R2X_RSC: {}", e),
            }
            continue;
        }
    }
}

/// Handle a SEND_REQ packet
fn handle_sender_request_passphrase_message(
    listener: &UdpSocket,
    addr: &SocketAddr,
    payload_str: &str,
    passphrase_generator: &PassphraseGenerator,
    client_map: &mut HashMap<Passphrase<'static>, FileInfo>,
) -> Result<()> {
    let payload: S2XRequestPassphraseMessage = serde_json::from_str(payload_str)?;

    let file_info = FileInfo {
        file_size: payload.file_size,
        file_name: payload.file_name,
        file_hash: payload.file_hash,
        created_at: current_unix_millis(),
        sender_host: payload.sender_host,
        sender_addr: *addr,
    };

    let passphrase = passphrase_generator.generate()
        .ok_or(NudgeError::PassphraseGenerationError)?;

    client_map.insert(passphrase.clone(), file_info);
    send_passphrase_to_sender(listener, addr, passphrase)
}

/// Send a SEND_ACK packet
fn send_passphrase_to_sender(
    listener: &UdpSocket,
    addr: &SocketAddr,
    passphrase: Passphrase<'static>
) -> Result<()> {
    let response_payload = X2SPassphraseProvidedMessage { passphrase };
    let response = format!("X2S_PPM {}\n", serde_json::to_string(&response_payload)?);
    listener.send_to(response.as_bytes(), addr)?;

    Ok(())
}

fn handle_receiver_request_file_info(
    listener: &UdpSocket,
    addr: &SocketAddr,
    payload_str: &str,
    client_map: &HashMap<Passphrase<'static>, FileInfo>,
) -> Result<()> {
    let payload: R2XRequestFileInfoMessage = serde_json::from_str(payload_str)?;

    if let Some(file_info) = client_map.get(&payload.passphrase) {
        send_file_info_to_receiver(listener, addr, file_info)
    } else {
        Err(NudgeError::PassphraseNotFound)
    }
}

fn send_file_info_to_receiver(
    listener: &UdpSocket,
    addr: &SocketAddr,
    file_info: &FileInfo
) -> Result<()> {
    let response = format!("X2R_AFI {}\n", serde_json::to_string(file_info)?);
    listener.send_to(response.as_bytes(), addr)?;
    Ok(())
}

fn handle_receiver_accept(listener: &UdpSocket,
                          addr: &SocketAddr,
                          payload_str: &str,
                            client_map: &mut HashMap<Passphrase<'static>, FileInfo>,
) -> Result<()> {
    let payload: R2XRequestSenderConnectionMessage = serde_json::from_str(payload_str)?;

    let file_info = match client_map.get_mut(&payload.passphrase) {
        Some(file_info) => file_info,
        None => return Err(NudgeError::PassphraseNotFound),
    };

    // make sure the file hash matches
    if file_info.file_hash == payload.file_hash {
        println!(
            "File hash matches, sending sender ({}) to receiver ({})",
            file_info.sender_addr, addr
        );

        // Clone the necessary info before removing the entry
        let sender_addr = file_info.sender_addr.clone();

        client_map.remove(&payload.passphrase);

        send_sender_connect_to_receiver(listener, &sender_addr, addr, payload.receiver_host)
    } else {
        Err(NudgeError::PassphraseNotFound)
    }
}

fn send_sender_connect_to_receiver(listener: &UdpSocket,
                                   sender_addr: &SocketAddr,
                                   receiver_addr: &SocketAddr,
                                   sender_host: AnonymousString,
) -> Result<()> {
    let response_payload = X2SSenderConnectToReceiverMessage {
        receiver_addr: *receiver_addr,
        receiver_host: sender_host,
    };
    let response = format!("X2S_SCON {}\n", serde_json::to_string(&response_payload)?);
    listener.send_to(response.as_bytes(), sender_addr)?;

    Ok(())
}