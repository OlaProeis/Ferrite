//! JSON-RPC message framing over LSP stdio (`Content-Length` headers).

use serde_json::Value;
use std::io::{Read, Write};

/// Failure while reading or parsing a framed message.
#[derive(Debug)]
pub enum TransportError {
    InvalidHeader(String),
    Json(serde_json::Error),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::InvalidHeader(s) => write!(f, "invalid LSP header: {s}"),
            TransportError::Json(e) => write!(f, "JSON error: {e}"),
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TransportError::Json(e) => Some(e),
            TransportError::InvalidHeader(_) => None,
        }
    }
}

/// Serialize a JSON value as one LSP stdio message (header + UTF-8 body).
pub fn encode_lsp_message(value: &Value) -> Result<Vec<u8>, serde_json::Error> {
    let body = serde_json::to_vec(value)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut out = header.into_bytes();
    out.extend_from_slice(&body);
    Ok(out)
}

/// Write a framed message to a writer (blocking).
pub fn write_lsp_message<W: Write>(writer: &mut W, value: &Value) -> Result<(), TransportError> {
    let buf = encode_lsp_message(value).map_err(TransportError::Json)?;
    writer
        .write_all(&buf)
        .map_err(|e| TransportError::InvalidHeader(e.to_string()))?;
    writer
        .flush()
        .map_err(|e| TransportError::InvalidHeader(e.to_string()))?;
    Ok(())
}

/// Incremental reader for inbound LSP messages from a byte stream (e.g. server stdout).
pub struct MessageReader {
    buf: Vec<u8>,
}

impl Default for MessageReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageReader {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn push_bytes(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// If a full message is available, remove it from the buffer and return the parsed JSON.
    pub fn try_next_message(&mut self) -> Result<Option<Value>, TransportError> {
        let Some(header_len) = find_header_end(&self.buf) else {
            return Ok(None);
        };

        let header_str = std::str::from_utf8(&self.buf[..header_len])
            .map_err(|e| TransportError::InvalidHeader(format!("header utf-8: {e}")))?;
        let content_length = parse_content_length(header_str)?;
        let body_start = header_len + 4;
        let total_needed = body_start + content_length;
        if self.buf.len() < total_needed {
            return Ok(None);
        }

        let body = &self.buf[body_start..total_needed];
        let value: Value = serde_json::from_slice(body).map_err(TransportError::Json)?;
        self.buf.drain(..total_needed);
        Ok(Some(value))
    }
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length(header: &str) -> Result<usize, TransportError> {
    for line in header.split("\r\n") {
        let line = line.trim();
        let rest = line
            .strip_prefix("Content-Length:")
            .or_else(|| line.strip_prefix("content-length:"));
        if let Some(num) = rest {
            let n = num.trim().parse::<usize>().map_err(|e| {
                TransportError::InvalidHeader(format!("Content-Length parse error: {e}"))
            })?;
            return Ok(n);
        }
    }
    Err(TransportError::InvalidHeader(
        "missing Content-Length header".into(),
    ))
}

/// Read one complete message from a blocking reader (used by background loops).
pub fn read_lsp_message<R: Read>(
    reader: &mut R,
    acc: &mut MessageReader,
) -> Result<Value, TransportError> {
    loop {
        if let Some(v) = acc.try_next_message()? {
            return Ok(v);
        }
        let mut chunk = [0u8; 4096];
        let n = reader
            .read(&mut chunk)
            .map_err(|e| TransportError::InvalidHeader(e.to_string()))?;
        if n == 0 {
            return Err(TransportError::InvalidHeader(
                "unexpected EOF before complete message".into(),
            ));
        }
        acc.push_bytes(&chunk[..n]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn roundtrip_encode_decode() {
        let v = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
        let bytes = encode_lsp_message(&v).unwrap();
        let mut r = MessageReader::new();
        r.push_bytes(&bytes);
        let out = r.try_next_message().unwrap().unwrap();
        assert_eq!(out, v);
        assert!(r.buf.is_empty());
    }

    #[test]
    fn chunked_arrival() {
        let v = json!({"jsonrpc":"2.0","id":null});
        let bytes = encode_lsp_message(&v).unwrap();
        let mut r = MessageReader::new();
        let mid = bytes.len() / 2;
        r.push_bytes(&bytes[..mid]);
        assert!(r.try_next_message().unwrap().is_none());
        r.push_bytes(&bytes[mid..]);
        assert_eq!(r.try_next_message().unwrap().unwrap(), v);
    }

    #[test]
    fn parse_content_length_line() {
        let h = "Content-Length: 12\r\n";
        assert_eq!(parse_content_length(h).unwrap(), 12);
    }
}
