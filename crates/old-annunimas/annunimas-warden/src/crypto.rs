// sigil: REPAIR
//! Crypto utilities for WARDEN
//!
//! Simplified placeholder - expand with real sodiumoxide later.

use base64::{engine::general_purpose, Engine as _};
use serde_json::Value;

pub struct Crypto {
    // Placeholder for sodiumoxide keys
    _warden_pubkey: Vec<u8>,
}

impl Crypto {
    pub fn new(warden_pubkey_base64: &str) -> anyhow::Result<Self> {
        let pubkey_bytes = general_purpose::STANDARD
            .decode(warden_pubkey_base64)
            .map_err(|e| anyhow::anyhow!("Invalid base64: {}", e))?;

        Ok(Self {
            _warden_pubkey: pubkey_bytes,
        })
    }

    /// Encrypt a report (placeholder)
    pub fn encrypt_report(&self, report: &Value) -> anyhow::Result<String> {
        let plaintext = report.to_string();
        let encoded = general_purpose::STANDARD.encode(plaintext.as_bytes());
        Ok(encoded)
    }

    /// Decrypt a report (placeholder)
    pub fn decrypt_report(&self, encrypted_base64: &str) -> anyhow::Result<Value> {
        let bytes = general_purpose::STANDARD
            .decode(encrypted_base64)
            .map_err(|e| anyhow::anyhow!("Invalid base64: {}", e))?;

        let plaintext =
            String::from_utf8(bytes).map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e))?;

        serde_json::from_str(&plaintext).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))
    }
}
