//! Prometheus text exposition for governance snapshots collected by `arda-governance`.
//!
//! Aule owns this scrape-compatible rendering surface. The governance library
//! deliberately owns no HTTP server.

use arda_governance::GovernanceMetricsSnapshot;
use std::collections::BTreeSet;
use std::fmt::Write;

pub fn render_governance_prometheus(snapshot: &GovernanceMetricsSnapshot) -> String {
    let mut output = String::new();
    let mut declared = BTreeSet::new();
    for counter in &snapshot.counters {
        let name = metric_name(&counter.name);
        if declared.insert(name.clone()) {
            let _ = writeln!(output, "# HELP {name} Arda governance counter snapshot");
            let _ = writeln!(output, "# TYPE {name} counter");
        }
        let _ = writeln!(
            output,
            "{}{} {}",
            name,
            render_labels(
                counter
                    .labels
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
            ),
            counter.value
        );
    }
    for histogram in &snapshot.histograms {
        let name = metric_name(&histogram.name);
        let _ = writeln!(
            output,
            "# HELP {name} Arda governance normalized score histogram"
        );
        let _ = writeln!(output, "# TYPE {name} histogram");
        for bucket in &histogram.buckets {
            let _ = writeln!(
                output,
                "{name}_bucket{{le=\"{}\"}} {}",
                bucket.upper_bound, bucket.cumulative_count
            );
        }
        let _ = writeln!(output, "{name}_bucket{{le=\"+Inf\"}} {}", histogram.count);
        let _ = writeln!(output, "{name}_sum {}", histogram.sum);
        let _ = writeln!(output, "{name}_count {}", histogram.count);
    }
    if let Some(writer) = snapshot.bacon_lite_writer {
        for (name, value) in [
            ("accepted_events", writer.accepted_events),
            ("written_events", writer.written_events),
            ("dropped_events", writer.dropped_events),
            ("failed_events", writer.failed_events),
            ("write_errors", writer.write_errors),
        ] {
            let _ = writeln!(
                output,
                "arda_governance_bacon_lite_writer_events{{kind=\"{name}\"}} {value}"
            );
        }
    }
    output
}

fn metric_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == ':' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn render_labels<'a>(labels: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    let values = labels
        .map(|(key, value)| format!("{}=\"{}\"", metric_name(key), escape_label(value)))
        .collect::<Vec<_>>();
    if values.is_empty() {
        String::new()
    } else {
        format!("{{{}}}", values.join(","))
    }
}

fn escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}
