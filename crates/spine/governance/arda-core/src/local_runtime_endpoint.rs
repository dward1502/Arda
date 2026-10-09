//! Loopback Manwe endpoint selected by a release's services.toml.
use anyhow::{bail, Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalRuntimeEndpoint {
    pub base_url: String,
    pub health_url: String,
}

impl LocalRuntimeEndpoint {
    /// Require a single Manwe declaration whose explicit IPv4 loopback bind,
    /// port and health probe agree. No environment or default endpoint fallback.
    pub fn from_services_toml(text: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct Manifest {
            #[serde(rename = "service", alias = "services", default)]
            services: Vec<Entry>,
        }
        #[derive(Deserialize)]
        struct Entry {
            name: String,
            start: Option<Start>,
            health: Option<Health>,
        }
        #[derive(Deserialize)]
        struct Start {
            #[serde(default)]
            args: Vec<String>,
        }
        #[derive(Deserialize)]
        struct Health {
            url: String,
            path: Option<String>,
        }
        let manifest: Manifest = toml::from_str(text).context("invalid services manifest")?;
        let mut matches = manifest
            .services
            .iter()
            .filter(|entry| entry.name == "manwe");
        let entry = matches
            .next()
            .context("missing Manwe endpoint declaration")?;
        if matches.next().is_some() {
            bail!("duplicate Manwe endpoint declaration");
        }
        let args = &entry.start.as_ref().context("missing Manwe start")?.args;
        let flag = |name: &str| -> Result<&str> {
            let mut values = Vec::new();
            for (i, arg) in args.iter().enumerate() {
                if arg == name {
                    values.push(
                        args.get(i + 1)
                            .context("missing endpoint argument")?
                            .as_str(),
                    );
                } else if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
                    values.push(value);
                }
            }
            if values.len() != 1 {
                bail!("require one explicit {name}");
            }
            Ok(values[0])
        };
        let bind = flag("--bind")?;
        let address: std::net::Ipv4Addr =
            bind.parse().context("Manwe bind must be literal IPv4")?;
        if !address.is_loopback() || address.to_string() != bind {
            bail!("Manwe endpoint must use canonical IPv4 loopback");
        }
        let port: u16 = flag("--port")?.parse().context("invalid Manwe port")?;
        if port == 0 {
            bail!("Manwe endpoint requires a fixed nonzero port");
        }
        let base_url = format!("http://{address}:{port}");
        let health_url = format!("{base_url}/healthz");
        let health = entry.health.as_ref().context("missing Manwe health")?;
        let declared = match &health.path {
            Some(path) => format!("{}{path}", health.url.trim_end_matches('/')),
            None => health.url.clone(),
        };
        if declared != health_url {
            bail!("Manwe bind and health endpoint disagree");
        }
        Ok(Self {
            base_url,
            health_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn registry(bind: &str, port: &str, health: &str) -> String {
        format!(
            r#"[[service]]
name = "manwe"
start = {{ command = "manwe", args = ["--bind", "{bind}", "--port", "{port}"] }}
health = {{ url = "{health}" }}
"#
        )
    }
    #[test]
    fn provisioning_parser_matches_rust_endpoint_contract() {
        use std::{
            io::Write,
            process::{Command, Stdio},
        };
        let good = registry("127.0.0.1", "17171", "http://127.0.0.1:17171/healthz");
        let mut cases = vec![
            good.clone(),
            good.replace("[[service]]", "[[services]]"),
            good.replace(
                "\"--bind\", \"127.0.0.1\", \"--port\", \"17171\"",
                "\"--bind=127.0.0.1\", \"--port=17171\"",
            ),
            good.replace(
                "url = \"http://127.0.0.1:17171/healthz\"",
                "url = \"http://127.0.0.1:17171/\", path = \"/healthz\"",
            ),
            format!("{good}\n{good}"),
            String::new(),
            good.replace("\"--port\"", "\"--port\", \"17171\", \"--port\""),
            good.replace("\"--port\", \"17171\"", "\"--port\", 17171"),
            format!("{good}\n[[services]]\nname='other'"),
        ];
        for (bind, port, health) in [
            ("0.0.0.0", "17171", "http://0.0.0.0:17171/healthz"),
            ("localhost", "17171", "http://localhost:17171/healthz"),
            ("127.0.0.1", "0", "http://127.0.0.1:0/healthz"),
            ("127.0.0.1", "65536", "http://127.0.0.1:65536/healthz"),
            ("127.0.0.1", "+17171", "http://127.0.0.1:17171/healthz"),
            ("127.0.0.1", "017171", "http://127.0.0.1:17171/healthz"),
            ("127.0.0.1", "17171", "http://127.0.0.1:7171/healthz"),
            ("127.0.0.1", "17171", "http://user@127.0.0.1:17171/healthz"),
        ] {
            cases.push(registry(bind, port, health));
        }
        let expected: Vec<_> = cases
            .iter()
            .map(|text| {
                LocalRuntimeEndpoint::from_services_toml(text)
                    .ok()
                    .map(|endpoint| endpoint.base_url)
            })
            .collect();
        let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../../scripts/configure-retained-runtime.py");
        let code = "import importlib.util,json,sys\ns=importlib.util.spec_from_file_location('g',sys.argv[1]);m=importlib.util.module_from_spec(s);s.loader.exec_module(m)\nresults=[]\nfor case in json.load(sys.stdin):\n try: results.append(m.local_runtime_endpoint(case.encode()))\n except (ValueError,TypeError,KeyError): results.append(None)\nprint(json.dumps(results))";
        let mut child = Command::new("python3")
            .args(["-I", "-S", "-B", "-c", code])
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&cases).unwrap())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        let actual: Vec<Option<String>> = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn selected_endpoint_matches_process_and_probe() {
        let endpoint = LocalRuntimeEndpoint::from_services_toml(&registry(
            "127.0.0.1",
            "17171",
            "http://127.0.0.1:17171/healthz",
        ))
        .unwrap();
        assert_eq!(endpoint.base_url, "http://127.0.0.1:17171");
        assert_eq!(endpoint.health_url, "http://127.0.0.1:17171/healthz");
    }
    #[test]
    fn endpoint_refuses_conflicts_and_missing_authority() {
        for text in [
            registry("127.0.0.1", "17171", "http://127.0.0.1:7171/healthz"),
            registry("0.0.0.0", "17171", "http://0.0.0.0:17171/healthz"),
            registry("localhost", "17171", "http://localhost:17171/healthz"),
            registry("127.0.0.1", "0", "http://127.0.0.1:0/healthz"),
            registry("127.0.0.1", "17171", "http://user@127.0.0.1:17171/healthz"),
            registry("127.0.0.1", "17171", "http://127.0.0.1:17171/healthz?x=y"),
            registry("127.0.0.1", "17171", "https://127.0.0.1:17171/healthz"),
            String::new(),
        ] {
            assert!(
                LocalRuntimeEndpoint::from_services_toml(&text).is_err(),
                "{text}"
            );
        }
    }
    #[test]
    fn endpoint_accepts_legacy_split_and_rejects_duplicates() {
        let text = registry("127.0.0.1", "17171", "http://127.0.0.1:17171/healthz");
        let split = text.replace(
            "url = \"http://127.0.0.1:17171/healthz\"",
            "url = \"http://127.0.0.1:17171\", path = \"/healthz\"",
        );
        assert_eq!(
            LocalRuntimeEndpoint::from_services_toml(&text).unwrap(),
            LocalRuntimeEndpoint::from_services_toml(&split).unwrap()
        );
        assert!(LocalRuntimeEndpoint::from_services_toml(&format!("{text}\n{text}")).is_err());
        assert!(LocalRuntimeEndpoint::from_services_toml(
            &text.replace("\"--bind\"", "\"--bind\", \"127.0.0.1\", \"--bind\"")
        )
        .is_err());
    }
}
