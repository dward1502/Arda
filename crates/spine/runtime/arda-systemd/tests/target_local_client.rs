use arda_systemd::{parse_list_units, SystemdClient, SystemdError, UnitKind};

struct FakeClient {
    output: Result<&'static str, SystemdError>,
}

impl SystemdClient for FakeClient {
    fn list_units_raw(&self, _pattern: &str) -> Result<String, SystemdError> {
        match &self.output {
            Ok(output) => Ok((*output).to_owned()),
            Err(SystemdError::Exit { code, stderr }) => Err(SystemdError::Exit {
                code: *code,
                stderr: stderr.clone(),
            }),
            Err(SystemdError::Io(error)) => Err(SystemdError::Io(std::io::Error::new(
                error.kind(),
                error.to_string(),
            ))),
        }
    }
}

#[test]
fn parses_mixed_systemd_units_into_typed_kinds() {
    let raw = "\
arda-charon.service loaded active running Charon\n\
arda-maintenance.timer loaded active waiting Maintenance timer\n\
arda-gateway.socket loaded inactive listening Gateway socket\n\
arda-data.mount loaded active mounted Data mount\n";

    let units = parse_list_units(raw);

    assert_eq!(units.len(), 4);
    assert_eq!(units[0].name, "arda-charon.service");
    assert_eq!(units[0].kind, UnitKind::Service);
    assert_eq!(units[0].active, "active");
    assert_eq!(units[1].kind, UnitKind::Timer);
    assert_eq!(units[1].sub, "waiting");
    assert_eq!(units[2].kind, UnitKind::Socket);
    assert_eq!(units[3].kind, UnitKind::Other);
}

#[test]
fn ignores_malformed_list_unit_rows_without_panicking() {
    let raw = "\
arda-good.service loaded active running Good\n\
malformed-row\n\
arda-no-sub.service loaded active\n\
arda-failed.service loaded failed failed Failed\n";

    let units = parse_list_units(raw);

    assert_eq!(units.len(), 2);
    assert_eq!(units[0].name, "arda-good.service");
    assert_eq!(units[1].name, "arda-failed.service");
    assert_eq!(units[1].active, "failed");
}

#[test]
fn trait_default_method_parses_raw_client_output() -> Result<(), SystemdError> {
    let client = FakeClient {
        output: Ok("arda-athena.service loaded inactive dead Athena\n"),
    };

    let units = client.list_units("arda-*")?;

    assert_eq!(units.len(), 1);
    assert_eq!(units[0].name, "arda-athena.service");
    assert_eq!(units[0].kind, UnitKind::Service);
    assert_eq!(units[0].active, "inactive");
    Ok(())
}

#[test]
fn trait_default_method_preserves_systemctl_exit_errors() {
    let client = FakeClient {
        output: Err(SystemdError::Exit {
            code: 42,
            stderr: "systemctl unavailable".to_owned(),
        }),
    };

    let result = client.list_units("arda-*");

    assert!(matches!(
        result,
        Err(SystemdError::Exit {
            code: 42,
            stderr
        }) if stderr == "systemctl unavailable"
    ));
}
