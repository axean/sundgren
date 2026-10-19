//! Reading text files the way the prototype did, with Python's error messages.

use std::io;
use std::path::Path;

#[derive(Debug)]
pub enum ReadError {
    Os(io::Error),
    Decode(String),
}

/// `Path.read_text(encoding="utf-8")`: strict UTF-8, universal newlines.
pub fn read_text(path: &Path) -> Result<String, ReadError> {
    let bytes = std::fs::read(path).map_err(ReadError::Os)?;
    let text = String::from_utf8(bytes).map_err(|e| {
        let bytes = e.as_bytes();
        ReadError::Decode(utf8_error_message(bytes, e.utf8_error()))
    })?;
    if !text.contains('\r') {
        return Ok(text);
    }
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Python's UnicodeDecodeError text for strict UTF-8.
pub fn utf8_error_message(bytes: &[u8], err: std::str::Utf8Error) -> String {
    let start = err.valid_up_to();
    let (len, reason) = match err.error_len() {
        None => (bytes.len() - start, "unexpected end of data"),
        Some(n) => {
            let b = bytes[start];
            let bad_start = (0x80..0xC2).contains(&b) || b >= 0xF5;
            (
                n,
                if bad_start {
                    "invalid start byte"
                } else {
                    "invalid continuation byte"
                },
            )
        }
    };
    if len == 1 {
        format!(
            "'utf-8' codec can't decode byte 0x{:02x} in position {start}: {reason}",
            bytes[start]
        )
    } else {
        format!(
            "'utf-8' codec can't decode bytes in position {start}-{}: {reason}",
            start + len - 1
        )
    }
}

/// `exc.strerror` of an OS error.
pub fn strerror(e: &io::Error) -> String {
    let text = e.to_string();
    match e.raw_os_error() {
        Some(code) => text
            .strip_suffix(&format!(" (os error {code})"))
            .unwrap_or(&text)
            .to_owned(),
        None => text,
    }
}
