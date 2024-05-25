use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use crate::passphrase::Passphrase;
use crate::utils::AnonymousString;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    /// Size of the file in bytes
    pub(crate) file_size: u64,

    /// Name of the file
    pub(crate) file_name: String,

    /// Hash of the file (optional)
    pub(crate) file_hash: AnonymousString,

    /// Hostname of the sender (optional)
    pub(crate) sender_host: AnonymousString,

    /// Timestamp when the file was created
    pub(crate) created_at: u64,

    /// Address of the sender
    pub(crate) sender_addr: SocketAddr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct S2XRequestPassphraseMessage {
    /// Size of the file in bytes
    pub(crate) file_size: u64,

    /// Name of the file
    pub(crate) file_name: String,

    /// Hash of the file (optional)
    pub(crate) file_hash: AnonymousString,

    /// Hostname of the sender (optional)
    pub(crate) sender_host: AnonymousString,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct X2SPassphraseProvidedMessage {
    /// Passphrase to access the file
    pub(crate) passphrase: Passphrase<'static>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct R2XRequestFileInfoMessage {
    /// Passphrase to access the file
    pub(crate) passphrase: Passphrase<'static>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct R2XRequestSenderConnectionMessage {
    /// Passphrase to access the file
    pub(crate) passphrase: Passphrase<'static>,

    /// Size of the file in bytes (optional)
    pub(crate) file_hash: AnonymousString,

    /// Hostname of the receiver (optional)
    pub(crate) receiver_host: AnonymousString,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct X2SSenderConnectToReceiverMessage {
    /// Address of the receiver
    pub(crate) receiver_addr: SocketAddr,
    pub(crate) receiver_host: AnonymousString,
}
