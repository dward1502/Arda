use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Transaction};

/// Fixed ISO-8601 time durations only; calendar periods need timezone policy.
pub(super) fn recurrence_ms(value: &str) -> Result<i64> {
    let body = value
        .strip_prefix("PT")
        .ok_or_else(|| anyhow!("recurrence must be a fixed PT duration"))?;
    let mut digits = String::new();
    let mut total = 0_i64;
    let mut previous = 0;
    for ch in body.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        let (order, multiplier) = match ch {
            'H' => (1, 3_600_000_i64),
            'M' => (2, 60_000_i64),
            'S' => (3, 1_000_i64),
            _ => bail!("unsupported recurrence component"),
        };
        if order <= previous || digits.is_empty() {
            bail!("invalid recurrence component order");
        }
        let amount = digits
            .parse::<i64>()
            .context("recurrence amount overflow")?;
        total = amount
            .checked_mul(multiplier)
            .and_then(|part| total.checked_add(part))
            .ok_or_else(|| anyhow!("recurrence duration overflow"))?;
        digits.clear();
        previous = order;
    }
    if !digits.is_empty() || total <= 0 {
        bail!("recurrence must be a positive fixed duration");
    }
    Ok(total)
}

/// Consume wakes in the admission transaction, never reopening terminal work.
/// Recurrence means periodic wake of this unfinished objective, not permission
/// to repeat completed actions. Coalesce missed ticks onto the original phase.
pub(super) fn consume_due(transaction: &Transaction<'_>, now_ms: i64) -> Result<()> {
    let due = {
        let mut statement = transaction.prepare(
            "SELECT s.id, s.next_wake_ms, s.recurrence FROM schedules s
             JOIN objectives o ON o.id = s.objective_id
             WHERE o.state IN ('approved', 'running') AND s.next_wake_ms <= ?1
               AND NOT EXISTS (SELECT 1 FROM schedule_errors e WHERE e.schedule_id = s.id)
               AND (s.recurrence IS NOT NULL OR NOT EXISTS
                    (SELECT 1 FROM schedule_wakes w WHERE w.schedule_id = s.id))
             ORDER BY s.next_wake_ms, s.id LIMIT 256",
        )?;
        let rows = statement.query_map([now_ms], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, due_ms, recurrence) in due {
        if let Some(recurrence) = recurrence {
            let next = recurrence_ms(&recurrence).and_then(|interval| {
                let interval = i128::from(interval);
                let elapsed = i128::from(now_ms) - i128::from(due_ms);
                let next = i128::from(due_ms) + (elapsed / interval + 1) * interval;
                i64::try_from(next).context("next schedule wake overflow")
            });
            let next = match next {
                Ok(next) => next,
                Err(error) => {
                    // Historical formats and exhausted timestamps fail only this
                    // schedule. Preserve its payload and do not authorize admission.
                    transaction.execute(
                        "INSERT INTO schedule_errors (schedule_id, error, observed_at_ms)
                         VALUES (?1, ?2, ?3)",
                        params![id, error.to_string(), now_ms],
                    )?;
                    tracing::warn!(schedule_id = %id, %error, "objective schedule quarantined");
                    continue;
                }
            };
            transaction.execute(
                "UPDATE schedules SET next_wake_ms = ?2, updated_at_ms = ?3 WHERE id = ?1",
                params![id, next, now_ms],
            )?;
        }
        transaction.execute(
            "INSERT INTO schedule_wakes (schedule_id, last_wake_ms) VALUES (?1, ?2)
             ON CONFLICT(schedule_id) DO UPDATE SET last_wake_ms = excluded.last_wake_ms",
            params![id, now_ms],
        )?;
    }
    Ok(())
}
