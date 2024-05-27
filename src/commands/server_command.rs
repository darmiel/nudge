use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::str;

use clap::Parser;
use crate::commands::RootOpts;

use crate::error::{NudgeError, Result};
use crate::error::NudgeError::UnknownCommand;
use crate::utils::passphrase::{Passphrase, PassphraseGenerator};
use crate::utils::{AnonymousString, current_unix_millis};
use crate::models::*;

#[derive(Parser, Debug)]
pub struct RelayServerOpts {}

pub fn run(root_opts: &RootOpts, _: &RelayServerOpts) -> Result<()> {
    let passphrase_generator = PassphraseGenerator::new()?;
    let mut client_map = HashMap::new();

    let bind_addr = format!("{}:{}", root_opts.relay_host, root_opts.relay_port);
    info!("Starting server on {}", bind_addr);

    let listener = UdpSocket::bind(&bind_addr)?;

    let mut buf = [0u8; 1024];

    loop {
        let (len, addr) = listener.recv_from(&mut buf)?;
        info!("Received {} bytes from {}", len, addr);

        let received_str = match str::from_utf8(&buf[..len]) {
            Ok(s) => s,
            Err(e) => {
                warn!("({}) Error converting to string: {}", addr, e);
                continue;
            }
        };
        info!("({}) Received Data: {:?}", addr, received_str);

        match handle_message(received_str, &listener, &addr, &passphrase_generator, &mut client_map) {
            Ok(_) => info!("Handled message without error"),
            Err(e) => {
                warn!("Handled message with error: {}", e);

                match send_error(&listener, &addr, &e.to_string()) {
                    Ok(_) => info!("Sent error message to client"),
                    Err(e) => error!("Cannot even send the error to the client: {}", e),
                }
            }
        }
    }
}

fn handle_message(
    received_str: &str,
    listener: &UdpSocket,
    addr: &SocketAddr,
    passphrase_generator: &PassphraseGenerator,
    client_map: &mut HashMap<Passphrase<'static>, FileInfo>,
) -> Result<()> {
    match received_str.split_whitespace().next() {
        // Sender -> Server; Request Passphrase
        Some("S2X_RP") => handle_sender_request_passphrase_message(
            &listener, &addr, &received_str[7..], &passphrase_generator, client_map,
        ),
        // Receiver -> Server; Request File Info
        Some("R2X_RFI") => handle_receiver_request_file_info(
            &listener, &addr, &received_str[8..], client_map,
        ),
        // Receiver -> Server; Accept Connection
        Some("R2X_RSC") => handle_receiver_accept(
            &listener, &addr, &received_str[8..], client_map,
        ),
        _ => Err(UnknownCommand)
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

fn send_passphrase_to_sender(
    listener: &UdpSocket,
    addr: &SocketAddr,
    passphrase: Passphrase<'static>,
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
    file_info: &FileInfo,
) -> Result<()> {
    let response = format!("X2R_AFI {}\n", serde_json::to_string(file_info)?);
    listener.send_to(response.as_bytes(), addr)?;
    Ok(())
}

fn handle_receiver_accept(
    listener: &UdpSocket,
    addr: &SocketAddr,
    payload_str: &str,
    client_map: &mut HashMap<Passphrase<'static>, FileInfo>,
) -> Result<()> {
    let payload: R2XRequestSenderConnectionMessage = serde_json::from_str(payload_str)?;

    // check if the passphrase exists
    let file_info = match client_map.get_mut(&payload.passphrase) {
        Some(file_info) => file_info,
        None => return Err(NudgeError::PassphraseNotFound),
    };

    // make sure the file hash matches
    if file_info.file_hash == payload.file_hash {
        info!(
            "({}) File hash matches, sending sender ({}) to receiver ({})",
            addr, file_info.sender_addr, addr
        );

        // Clone the necessary info before removing the entry
        let sender_addr = file_info.sender_addr;

        client_map.remove(&payload.passphrase);

        send_sender_connect_to_receiver(listener, &sender_addr, addr, payload.receiver_host)
    } else {
        Err(NudgeError::PassphraseNotFound)
    }
}

fn send_sender_connect_to_receiver(
    listener: &UdpSocket,
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

fn send_error(listener: &UdpSocket, addr: &SocketAddr, error: &str) -> Result<()> {
    let response = format!("ERROR {}\n", error);
    listener.send_to(response.as_bytes(), addr)?;
    Ok(())
}