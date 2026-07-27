// sigil: REPAIR
use serde_json::json;
use std::collections::HashMap;

pub struct PlutusLedger {
    entries: HashMap<String, f64>,
    credit_total: f64,
    credit_events: u64,
    last_credit_account: Option<String>,
    last_credit_amount: Option<f64>,
}

impl PlutusLedger {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            credit_total: 0.0,
            credit_events: 0,
            last_credit_account: None,
            last_credit_amount: None,
        }
    }

    pub fn credit(&mut self, account: &str, amount: f64) {
        *self.entries.entry(account.to_owned()).or_insert(0.0) += amount;
        self.credit_total += amount;
        self.credit_events += 1;
        self.last_credit_account = Some(account.to_owned());
        self.last_credit_amount = Some(amount);
    }

    pub fn balance(&self, account: &str) -> f64 {
        *self.entries.get(account).unwrap_or(&0.0)
    }

    pub fn credit_total(&self) -> f64 {
        self.credit_total
    }

    pub fn credit_events(&self) -> u64 {
        self.credit_events
    }

    pub fn last_credit_account(&self) -> Option<&str> {
        self.last_credit_account.as_deref()
    }

    pub fn last_credit_amount(&self) -> Option<f64> {
        self.last_credit_amount
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
            "credit_total": self.credit_total,
            "credit_events": self.credit_events,
            "last_credit_account": self.last_credit_account,
            "last_credit_amount": self.last_credit_amount,
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
        self.credit_total = snapshot
            .get("credit_total")
            .and_then(|v| v.as_f64())
            .unwrap_or(self.credit_total);
        self.credit_events = snapshot
            .get("credit_events")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.credit_events);
        self.last_credit_account = snapshot
            .get("last_credit_account")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        self.last_credit_amount = snapshot.get("last_credit_amount").and_then(|v| v.as_f64());
    }
}

impl Default for PlutusLedger {
    fn default() -> Self {
        Self::new()
    }
}
