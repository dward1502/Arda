use super::CapturedRuntime;
use anyhow::{bail, Result};
use arda_engine::objectives::{
    runtime_operation::RuntimeOperation,
    runtime_policy::{GrantAccess, GrantKind, GrantRole},
};
use std::{os::fd::AsRawFd, process::Command};
impl CapturedRuntime {
    pub fn operation_argv(&self, operation: &RuntimeOperation) -> Result<Vec<String>> {
        operation.validate()?;
        // The owned bootstrap validates profile bytes before runtime initialization.
        if matches!(
            operation,
            RuntimeOperation::Chat { .. } | RuntimeOperation::Export { .. }
        ) {
            let home = std::path::Path::new(&self.policy.policy().fixed_environment["HERMES_HOME"]);
            if home
                .parent()
                .and_then(|p| p.file_name())
                .is_none_or(|p| p != "profiles")
                || !self.policy.policy().grants.iter().any(|grant| {
                    grant.role == GrantRole::ProfileInput
                        && grant.kind == GrantKind::File
                        && grant.access == GrantAccess::ReadOnly
                        && grant.destination == home.join("config.yaml")
                })
            {
                bail!("chat/export requires an explicit readonly dedicated profile");
            }
        }
        let mut argv = vec![
            self.policy
                .policy()
                .entrypoint
                .interpreter
                .to_string_lossy()
                .into_owned(),
            "-I".into(),
            "-S".into(),
            "-B".into(),
            "-c".into(),
        ];
        if let RuntimeOperation::VerifyArtifacts { paths } = operation {
            argv.push(include_str!("verifier.py").into());
            argv.push(serde_json::to_string(paths)?);
        } else {
            argv.push(
                concat!(
                    include_str!("profile_guard.py"),
                    "\n",
                    include_str!("bootstrap.py")
                )
                .into(),
            );
            argv.push(serde_json::to_string(
                &self.policy.policy().entrypoint.ordered_import_roots,
            )?);
            argv.push(
                match operation {
                    RuntimeOperation::Chat { .. } => "chat",
                    RuntimeOperation::Export { .. } => "export",
                    _ => "probe",
                }
                .into(),
            );
            argv.extend(operation.hermes_arguments()?);
        }
        Ok(argv)
    }
    pub fn mount_arguments(&self, command: &mut Command) -> Result<()> {
        let mut grants: Vec<_> = self.grants.iter().collect();
        grants.sort_by(|a, b| {
            a.identity
                .destination
                .components()
                .count()
                .cmp(&b.identity.destination.components().count())
                .then(a.identity.destination.cmp(&b.identity.destination))
        });
        for grant in grants {
            for alias in ["/bin", "/sbin", "/lib", "/lib64", "/proc", "/dev"] {
                if grant.identity.destination.starts_with(alias) {
                    bail!("grant collides with bootstrap-owned destination");
                }
            }
            command
                .arg(match grant.identity.access {
                    GrantAccess::ReadOnly => "--ro-bind-fd",
                    GrantAccess::ReadWrite => "--bind-fd",
                })
                .arg(grant.descriptor.as_raw_fd().to_string())
                .arg(&grant.identity.destination);
        }
        Ok(())
    }
    pub fn inherited_descriptors(&self) -> Vec<i32> {
        self.grants
            .iter()
            .map(|g| g.descriptor.as_raw_fd())
            .collect()
    }
}
