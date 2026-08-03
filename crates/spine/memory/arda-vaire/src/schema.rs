//! Version identifiers for durable Mnemosyne records and projections.

/// Current two-line JSONL episodic record schema.
pub const EPISODIC_SCHEMA_VERSION: &str = "arda.mnemosyne.episodic.v1";
/// Identifier assigned in memory when reading records written before explicit schemas.
pub const LEGACY_EPISODIC_SCHEMA_VERSION: &str = "arda.mnemosyne.episodic.legacy-v0";
/// Current status/continuity projection schema.
pub const CONTINUITY_SCHEMA_VERSION: &str = "arda.mnemosyne.continuity.v1";
/// Numeric version written to `MemoryRecord.extensions["persona.schema_version"]`.
pub const PERSONA_SCHEMA_VERSION: u32 = 1;
/// Stable identifier for the current persona projection shape.
pub const PERSONA_SCHEMA_ID: &str = "arda.vaire.persona.v1";
