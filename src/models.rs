use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use crate::passphrase::Passphrase;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub(crate) file_size: u64,
    pub(crate) file_name: String,
    pub(crate) file_hash: String,
    pub(crate) created_at: u64,
    pub(crate) sender_host: String,
    pub(crate) sender_addr: SocketAddr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileSendRequestPayload {
    pub(crate) file_size: u64,
    pub(crate) file_name: String,
    pub(crate) file_hash: String,
    pub(crate) sender_host: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileSendAckPayload {
    pub(crate) passphrase: Passphrase<'static>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileReceiveRequestPayload {
    pub(crate) passphrase: Passphrase<'static>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileReceiveAcceptPayload {
    pub(crate) passphrase: Passphrase<'static>,
    pub(crate) file_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SenderConnectToReceiverPayload {
    pub(crate) receiver_addr: SocketAddr,
}
