#[cfg(test)]
mod canary {
    #[test]
    fn human_surfaces_load() {
        let _ = arda_human::HumanIngestionRecord {
            contract: String::new(),
            source_path: String::new(),
            content_hash: String::new(),
            detected_status: String::new(),
            detected_authority: String::new(),
            source_type: String::new(),
            affected_agents: Vec::new(),
            affected_paths: Vec::new(),
            summary: String::new(),
            conflicts: Vec::new(),
            recommendation: String::new(),
            review_required: false,
            frontmatter_valid: false,
            missing_frontmatter_keys: Vec::new(),
            generated_at_utc: String::new(),
        };
        let _ = arda_human::HumanScanReport {
            schema_version: String::new(),
            generated_at_utc: String::new(),
            human_root: String::new(),
            output_path: String::new(),
            contradiction_path: None,
            scanned_total: 0,
            emitted_total: 0,
            contradiction_total: 0,
        };
        let _ = arda_human::scan_human_root;
        let _ = arda_human::classify_human_file;
    }
}
