use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::Read;
use std::time::SystemTime;
use console::style;
use dialoguer::theme::ColorfulTheme;
use gethostname::gethostname;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::error::{NudgeError, Result};

pub mod passphrase;
pub mod reliable_udp;
pub mod socket;
pub mod serialize;

#[cfg(debug_assertions)]
pub const DEFAULT_RELAY_HOST: &str = "127.0.0.1";
#[cfg(not(debug_assertions))]
pub const DEFAULT_RELAY_HOST: &str = "relay-1.nudge.d2a.io";

#[cfg(debug_assertions)]
pub const DEFAULT_RELAY_PORT: &str = "4000";
#[cfg(not(debug_assertions))]
pub const DEFAULT_RELAY_PORT: &str = "80";

pub const DEFAULT_CHUNK_SIZE: &str = "4096";

/// A wrapper around a string that can be displayed as "<anonymous>" if the string is None
#[derive(Serialize, Deserialize, Debug, Ord, PartialEq, PartialOrd, Eq, Clone)]
pub struct AnonymousString(pub Option<String>);

impl Display for AnonymousString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(ref hostname) = self.0 {
            f.write_str(hostname)
        } else {
            f.write_str("<anonymous>")
        }
    }
}

/// Returns the current time in milliseconds since the Unix epoch.
///
/// # Returns
///
/// `u64` - The current time in milliseconds.
pub fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Retrieves the hostname of the system.
///
/// # Returns
///
/// `Result<String>` - The hostname as a `String` on success, or a `NudgeError::HostnameError` on failure.
pub fn get_hostname() -> Result<String> {
    gethostname()
        .into_string()
        .map_err(|_| NudgeError::HostnameError)
}

/// Returns either an anonymous string or the hostname based on the `hide` parameter.
///
/// # Arguments
///
/// * `hide` - A boolean indicating whether to hide the hostname.
///
/// # Returns
///
/// `Result<AnonymousString>` - An `AnonymousString` containing either `None` if hidden or `Some(hostname)` if not.
pub fn hide_or_get_hostname(hide: bool) -> Result<AnonymousString> {
    Ok(AnonymousString(if hide { None } else { Some(get_hostname()?) }))
}

/// Hashes the contents of a file using the BLAKE3 hashing algorithm and resets the file's cursor to the start.
///
/// # Arguments
///
/// * `file` - A mutable reference to the file to be hashed.
///
/// # Returns
///
/// `Result<String>` - The hexadecimal hash string of the file contents.
pub fn hash_file_and_seek(file: &mut File) -> Result<String> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Creates a customized theme for prompts.
///
/// # Returns
///
/// `ColorfulTheme` - A theme with customized prompt, success, and error prefixes.
pub fn question_theme() -> ColorfulTheme {
    let mut theme = ColorfulTheme::default();
    theme.prompt_prefix = style("[?]".to_string()).for_stderr().dim();
    theme.success_prefix = style("[✔]".to_string()).for_stderr().bold().green();
    theme.error_prefix = style("[✗]".to_string()).for_stderr().bold().red();
    theme
}

/// Creates a new progress bar with a specified length and custom style.
///
/// # Arguments
///
/// * `length` - The total length of the progress bar.
///
/// # Returns
///
/// `ProgressBar` - A progress bar configured with a custom style and prefix.
pub fn new_downloader_progressbar(len: u64) -> ProgressBar {
    let progress_bar = ProgressBar::new(len)
        .with_prefix("[>]");
    progress_bar.set_style(ProgressStyle::with_template("{prefix:.orange} {elapsed_precise} :: |{wide_bar:.white/dim}| :: {bytes}/{total_bytes}")
        .unwrap()
        .progress_chars("█ :"));
    progress_bar
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
