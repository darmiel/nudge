use thiserror::Error;

#[derive(Error, Debug)]
pub enum NudgeError {
    #[error("IO Error")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse filesize")]
    ParseError(#[from] std::num::ParseIntError),

    #[error("UTF-8 conversion error")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("UTF-8 conversion error (2)")]
    Utf8Error2(#[from] std::str::Utf8Error),

    #[error("Buffer size exceeds the maximum allowed limit of 65532 bytes. Received: {0} bytes.")]
    BufferSizeLimitExceeded(usize),

    #[error("Data packet exceeds the maximum allowed limit of 65532 bytes. Received: {0} bytes.")]
    DataPacketLimitExceeded(usize),

    #[error("Failed to generate passphrase")]
    PassphraseGenerationError,

    #[error("Passphrase not found")]
    PassphraseNotFound,

    #[error("Failed to parse JSON")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Server returned error: {0}")]
    ServerError(String),

    #[error("Expected {0}, but received {1}")]
    ReceiveExpectationNotMet(String, String),

    #[error("Cannot get hostname")]
    HostnameError,

    #[error("Exited because --no-prompt was passed")]
    NoPromptExit,

    #[error("Hash mismatch! Expected: {0}, Received: {1}")]
    HashMismatch(String, String),

    #[error("Unknown command")]
    UnknownCommand,
}

pub type Result<T> = std::result::Result<T, NudgeError>;

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn test_io_error() {
        let io_error = io::Error::new(io::ErrorKind::Other, "some IO error");
        let nudge_error: NudgeError = io_error.into();
        assert!(matches!(nudge_error, NudgeError::Io(_)));
    }

    #[test]
    fn test_parse_int_error() {
        let parse_error = "not a number".parse::<usize>().unwrap_err();
        let nudge_error: NudgeError = parse_error.into();
        assert!(matches!(nudge_error, NudgeError::ParseError(_)));
    }

    #[test]
    fn test_utf8_error() {
        let bytes = vec![0xff, 0xff, 0xff];
        let utf8_error = String::from_utf8(bytes).unwrap_err();
        let nudge_error: NudgeError = utf8_error.into();
        assert!(matches!(nudge_error, NudgeError::Utf8Error(_)));
    }

    #[test]
    fn test_buffer_size_limit_exceeded() {
        let nudge_error = NudgeError::BufferSizeLimitExceeded(70000);
        assert!(matches!(nudge_error, NudgeError::BufferSizeLimitExceeded(70000)));
        if let NudgeError::BufferSizeLimitExceeded(size) = nudge_error {
            assert_eq!(size, 70000);
        }
    }

    #[test]
    fn test_data_packet_limit_exceeded() {
        let nudge_error = NudgeError::DataPacketLimitExceeded(70000);
        assert!(matches!(nudge_error, NudgeError::DataPacketLimitExceeded(70000)));
        if let NudgeError::DataPacketLimitExceeded(size) = nudge_error {
            assert_eq!(size, 70000);
        }
    }
}
