use super::*;
#[test]
fn chat_preserves_explicit_workspace_write_authority() {
    for writable in [false, true] {
        let value = serde_json::json!({
            "operation": "chat", "query": "test", "max_turns": 1,
            "toolsets": ["file"], "workspace_writable": writable
        });
        let operation: RuntimeOperation = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(operation).unwrap(), value);
    }
}
#[test]
fn artifact_argument_budget_counts_serialized_bytes() {
    let mut paths = vec!["a".repeat(4096); 16];
    let excess = serde_json::to_vec(&paths).unwrap().len() - 65_535;
    paths.last_mut().unwrap().truncate(4096 - excess);
    assert_eq!(serde_json::to_vec(&paths).unwrap().len(), 65_535);
    assert!(RuntimeOperation::VerifyArtifacts {
        paths: paths.clone()
    }
    .validate()
    .is_ok());
    paths.last_mut().unwrap().push('a');
    assert_eq!(serde_json::to_vec(&paths).unwrap().len(), 65_536);
    assert!(RuntimeOperation::VerifyArtifacts { paths }
        .validate()
        .is_err());
    // Escaped bytes, not just the unescaped UTF-8 length, consume argv space.
    assert!(RuntimeOperation::VerifyArtifacts {
        paths: vec!["\"".repeat(4096); 8]
    }
    .validate()
    .is_err());
    assert!(RuntimeOperation::VerifyArtifacts {
        paths: vec!["a".repeat(1200); 128]
    }
    .validate()
    .is_err());
}
#[test]
fn typed_operations_cannot_select_runtime_or_import_environment() {
    for field in ["executable", "environment", "profile", "argv"] {
        let mut value = serde_json::json!({"operation":"probe"});
        value[field] = serde_json::json!("untrusted-selector");
        assert!(serde_json::from_value::<RuntimeOperation>(value).is_err());
    }
    let chat = RuntimeOperation::Chat {
        query: "--profile=attacker".into(),
        max_turns: 3,
        toolsets: vec!["terminal".into()],
        workspace_writable: false,
    };
    let argv = chat.hermes_arguments().unwrap();
    assert!(argv.contains(&"--query=--profile=attacker".to_owned()));
    assert!(!argv.iter().any(|arg| arg.starts_with("--profile=")));
    assert!(RuntimeOperation::Chat {
        query: "test".into(),
        max_turns: 3,
        toolsets: vec!["terminal,delegation".into()],
        workspace_writable: false,
    }
    .validate()
    .is_err());
    assert!(RuntimeOperation::Export {
        session_id: "--profile=attacker".into()
    }
    .validate()
    .is_err());
    for path in ["/outside", "../outside", "a/../outside", "a//file", "."] {
        assert!(RuntimeOperation::VerifyArtifacts {
            paths: vec![path.into()]
        }
        .validate()
        .is_err());
    }
}
