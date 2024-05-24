use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use clap::Parser;

use crate::error::NudgeError;

#[derive(Parser, Debug)]
pub struct RelayServer {
    #[clap(short = 'x', long, default_value = "0.0.0.0")]
    host: String,

    #[clap(short, long, default_value = "4000")]
    port: u16,
}

impl RelayServer {
    pub fn run(&self) -> Result<(), NudgeError> {
        let bind_addr = format!("{}:{}", self.host, self.port);
        println!("Starting server on {}", bind_addr);

        let listener = UdpSocket::bind(&bind_addr)?;

        let mut client_map: HashMap<[u8; 200], SocketAddr> = HashMap::new();
        let mut buf = [0_u8; 200];

        loop {
            let (len, addr) = listener.recv_from(&mut buf)?;
            println!("Received {} bytes from {}", len, addr);

            if len != 200 {
                println!("Invalid packet size: {}", len);
                continue;
            }

            if let Some(other) = client_map.get(&buf) {
                println!("Found matching client: {}", addr);

                let mut bytes: &[u8] = addr.to_string().bytes().collect::<Vec<u8>>().leak();
                let mut addr_buf = [0u8; 200];
                for i in 0..bytes.len().min(200) {
                    addr_buf[i] = bytes[i];
                }
                bytes = other.to_string().bytes().collect::<Vec<u8>>().leak();
                let mut other_buf = [0u8; 200];
                for i in 0..bytes.len().min(200) {
                    other_buf[i] = bytes[i];
                }
                if listener.send_to(&addr_buf, other).is_ok()
                    && listener.send_to(&other_buf, addr).is_ok() {
                    println!("Exchanged addresses between {} and {}!", addr, other);
                }

                client_map.remove(&buf);
            } else {
                println!("No matching client found, adding to map");
                client_map.insert(buf, addr);
            }
        }
    }
}
