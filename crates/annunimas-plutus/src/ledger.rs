// sigil: REPAIR
use serde_json::json;
use std::collections::HashMap;

pub struct PlutusLedger {
    entries: HashMap<String, f64>,
}

impl PlutusLedger {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn credit(&mut self, account: &str, amount: f64) {
        *self.entries.entry(account.to_owned()).or_insert(0.0) += amount;
    }

    pub fn balance(&self, account: &str) -> f64 {
        *self.entries.get(account).unwrap_or(&0.0)
    }

    pub fn snapshot(&self) -> serde_json::Value {
        let mut rows = self
            .entries
            .iter()
            .map(|(account, balance)| json!({"account": account, "balance": balance}))
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            a.get("account")
                .and_then(|v| v.as_str())
                .cmp(&b.get("account").and_then(|v| v.as_str()))
        });
        json!({
            "accounts": rows,
            "accounts_total": self.entries.len(),
        })
    }

    pub fn restore_from_snapshot(&mut self, snapshot: &serde_json::Value) {
        self.entries.clear();
        let Some(rows) = snapshot.get("accounts").and_then(|v| v.as_array()) else {
            return;
        };
        for row in rows {
            let Some(account) = row.get("account").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(balance) = row.get("balance").and_then(|v| v.as_f64()) else {
                continue;
            };
            self.entries.insert(account.to_owned(), balance);
        }
    }
}

impl Default for PlutusLedger {
    fn default() -> Self {
        Self::new()
    }
}
