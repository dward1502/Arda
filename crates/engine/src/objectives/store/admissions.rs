//! Original admission, not a reconstruction from mutable objective state.
use super::*;

impl ObjectiveStore {
    /// Recover an existing admission without reading its original external inputs.
    /// Missing legacy snapshots fail closed; they are never synthesized on read.
    pub fn authenticated_admission(
        &self,
        ingress_key: &str,
        operator_id: &str,
    ) -> Result<Option<NewObjective>> {
        let row = self
            .connection()?
            .query_row(
                "SELECT o.id, o.operator_id, o.payload_digest, a.input_json
             FROM objectives o LEFT JOIN objective_admissions a ON a.objective_id=o.id
             WHERE o.ingress_key=?1",
                [ingress_key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()
            .context("read original objective admission")?;
        let Some((id, owner, digest, serialized)) = row else {
            return Ok(None);
        };
        if owner != operator_id {
            bail!("objective admission owner mismatch");
        }
        let serialized = serialized.ok_or_else(|| {
            anyhow!(
                "original objective admission unavailable; legacy state cannot be reconstructed"
            )
        })?;
        let input: NewObjective =
            serde_json::from_str(&serialized).context("decode original objective admission")?;
        if input.id != id
            || input.operator_id != owner
            || input.idempotency_key != ingress_key
            || digest_json(&input)? != digest
        {
            bail!("original objective admission integrity mismatch");
        }
        validate_objective(&input)?;
        Ok(Some(input))
    }
}
