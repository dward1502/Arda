//! Bounded JSON Lines framing helpers for `arda.project-adapter.v1`.

use std::io::{self, BufRead, Write};

use serde_json::{Map, Value};
use thiserror::Error;

pub const SCHEMA_VERSION: &str = "arda.project-adapter.v1";
pub const PROTOCOL_VERSION: &str = "1";
pub const MAX_LINE_BYTES: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("adapter frame exceeds {MAX_LINE_BYTES} bytes")]
    LineTooLarge,
    #[error("adapter stream ended with a partial frame")]
    PartialFrame,
    #[error("adapter frame is not valid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("adapter frame must be a JSON object")]
    NotAnObject,
    #[error("unsupported schema_version")]
    UnsupportedSchema,
    #[error("frame id must be a non-empty string")]
    InvalidId,
    #[error("frame type must be a non-empty string")]
    InvalidType,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

pub fn read_frame(reader: &mut impl BufRead) -> Result<Option<Map<String, Value>>, ProtocolError> {
    let mut bytes = Vec::new();
    let mut limited = std::io::Read::take(reader, (MAX_LINE_BYTES + 1) as u64);
    let read = limited.read_until(b'\n', &mut bytes)?;
    if read == 0 {
        return Ok(None);
    }
    if bytes.len() > MAX_LINE_BYTES {
        return Err(ProtocolError::LineTooLarge);
    }
    if !bytes.ends_with(b"\n") {
        return Err(ProtocolError::PartialFrame);
    }
    let value: Value = serde_json::from_slice(&bytes)?;
    let object = value.as_object().ok_or(ProtocolError::NotAnObject)?.clone();
    validate_envelope(&object)?;
    Ok(Some(object))
}

pub fn write_frame(
    writer: &mut impl Write,
    frame: &Map<String, Value>,
) -> Result<(), ProtocolError> {
    validate_envelope(frame)?;
    let payload = serde_json::to_vec(frame)?;
    if payload.len() + 1 > MAX_LINE_BYTES {
        return Err(ProtocolError::LineTooLarge);
    }
    writer.write_all(&payload)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

pub fn validate_envelope(frame: &Map<String, Value>) -> Result<(), ProtocolError> {
    if frame.get("schema_version").and_then(Value::as_str) != Some(SCHEMA_VERSION) {
        return Err(ProtocolError::UnsupportedSchema);
    }
    if frame
        .get("id")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(ProtocolError::InvalidId);
    }
    if frame
        .get("type")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err(ProtocolError::InvalidType);
    }
    Ok(())
}

pub fn negotiate_capabilities<'a>(
    requested: impl IntoIterator<Item = &'a str>,
    available: impl IntoIterator<Item = &'a str>,
) -> Vec<String> {
    let available = available
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    requested
        .into_iter()
        .filter(|capability| available.contains(capability))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use serde_json::json;

    fn frame() -> Map<String, Value> {
        json!({
            "schema_version": SCHEMA_VERSION,
            "id": "request-1",
            "type": "health"
        })
        .as_object()
        .unwrap()
        .clone()
    }

    #[test]
    fn valid_frame_round_trips() {
        let mut encoded = Vec::new();
        write_frame(&mut encoded, &frame()).unwrap();
        let decoded = read_frame(&mut Cursor::new(encoded)).unwrap().unwrap();
        assert_eq!(decoded, frame());
    }

    #[test]
    fn oversized_input_is_rejected_without_unbounded_read() {
        let mut input = vec![b'x'; MAX_LINE_BYTES + 1];
        input.push(b'\n');
        let error = read_frame(&mut Cursor::new(input)).unwrap_err();
        assert!(matches!(error, ProtocolError::LineTooLarge));
    }

    #[test]
    fn partial_frame_fails_closed() {
        let payload = serde_json::to_vec(&Value::Object(frame())).unwrap();
        let error = read_frame(&mut Cursor::new(payload)).unwrap_err();
        assert!(matches!(error, ProtocolError::PartialFrame));
    }

    #[test]
    fn capability_negotiation_is_ordered_and_deny_by_default() {
        assert_eq!(
            negotiate_capabilities(["read", "network", "write"], ["read", "write"]),
            ["read", "write"]
        );
    }
}
