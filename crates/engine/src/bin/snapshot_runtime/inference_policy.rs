//! Captured-profile inference policy, independent of provider naming and model family.
//! Traffic still goes only through the fixed local Manwe gateway; model payloads
//! cannot widen the selected route or obtain upstream credentials.
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

pub(super) struct Policy {
    pub(super) model: String,
    pub(super) provider: String,
    pub(super) routing: Value,
}
impl Policy {
    pub(super) fn from_profile(profile: &Value) -> Result<Self> {
        let custom = profile["custom_providers"]
            .as_array()
            .context("provider array missing")?;
        if custom.len() != 1 || custom[0]["base_url"] != "http://127.0.0.1:7171/v1" {
            bail!("broker requires exact local endpoint");
        }
        let model = profile["model"]["default"]
            .as_str()
            .context("model missing")?
            .to_owned();
        let routing = custom[0]["extra_body"]["routing"].clone();
        let provider = routing["force_provider_id"]
            .as_str()
            .context("provider missing")?
            .to_owned();
        // The captured profile owns route selection. Provider names do not
        // encode execution authority or force dependence on a particular model.
        let origin = routing["inference_origin"].as_str().unwrap_or_default();
        if provider.trim().is_empty()
            || model.trim().is_empty()
            || !matches!(origin, "local" | "cloud")
            || routing
                != json!({
                    "local_only": origin == "local", "inference_origin": origin,
                    "origin_preference": origin, "force_provider_id": provider,
                    "force_model_id": model, "allow_forced_provider_fallback": false,
                    "tool_use_required": true
                })
        {
            bail!("broker routing must pin a provider, model and explicit origin without fallback");
        }
        Ok(Self {
            model,
            provider,
            routing,
        })
    }
    pub(super) fn request(&self, bytes: &[u8]) -> Result<Value> {
        let mut body: Value = serde_json::from_slice(bytes)?;
        let fields = body.as_object().context("inference object required")?;
        let allowed = [
            "model",
            "messages",
            "tools",
            "tool_choice",
            "temperature",
            "top_p",
            "max_tokens",
            "max_completion_tokens",
            "stream",
            "stream_options",
            "stop",
            "seed",
            "frequency_penalty",
            "presence_penalty",
            "parallel_tool_calls",
            "response_format",
            "reasoning_effort",
            "routing",
        ];
        if fields.keys().any(|key| !allowed.contains(&key.as_str())) {
            bail!("unapproved request fields");
        }
        if body["model"] != self.model {
            bail!("unapproved request model");
        }
        if body.get("routing").is_some() && body["routing"] != self.routing {
            bail!("unapproved request routing");
        }
        // The trusted broker supplies routing even for SDK auxiliary requests.
        // Explicit conflicting selectors remain forbidden, never a fallback.
        body["routing"] = self.routing.clone();
        if !body["messages"].is_array() {
            bail!("messages array required");
        }
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(origin: &str, provider: &str) -> Value {
        json!({"model":{"default":"selected-model"},"custom_providers":[{
            "base_url":"http://127.0.0.1:7171/v1","extra_body":{"routing":{
                "local_only":origin=="local","inference_origin":origin,"origin_preference":origin,
                "force_provider_id":provider,"force_model_id":"selected-model",
                "allow_forced_provider_fallback":false,"tool_use_required":true}}}]})
    }
    #[test]
    fn pinned_local_and_cloud_routes_are_provider_independent() {
        for (origin, provider) in [
            ("local", "workstation"),
            ("cloud", "subscription"),
            ("cloud", "another-provider"),
        ] {
            let policy = Policy::from_profile(&profile(origin, provider)).unwrap();
            let request = policy
                .request(br#"{"model":"selected-model","messages":[]}"#)
                .unwrap();
            assert_eq!(request["routing"]["force_provider_id"], provider);
            assert_eq!(request["routing"]["inference_origin"], origin);
            assert_eq!(request["routing"]["allow_forced_provider_fallback"], false);
        }
    }
    #[test]
    fn profiles_cannot_widen_origin_or_fallback() {
        for (field, value) in [
            ("local_only", json!(true)),
            ("origin_preference", json!("auto")),
            ("inference_origin", json!("auto")),
            ("allow_forced_provider_fallback", json!(true)),
            ("tool_use_required", json!(false)),
            ("force_provider_id", json!("")),
            ("force_model_id", json!("different")),
            ("unknown", json!(true)),
        ] {
            let mut p = profile("cloud", "subscription");
            p["custom_providers"][0]["extra_body"]["routing"][field] = value;
            assert!(Policy::from_profile(&p).is_err(), "{field}");
        }
        let mut p = profile("cloud", "subscription");
        p["custom_providers"][0]["base_url"] = json!("https://external.invalid/v1");
        assert!(Policy::from_profile(&p).is_err());
        let policy = Policy::from_profile(&profile("cloud", "subscription")).unwrap();
        assert!(policy
            .request(br#"{"model":"different","messages":[]}"#)
            .is_err());
        assert!(policy
            .request(br#"{"model":"selected-model","messages":[],"routing":{}}"#)
            .is_err());
    }

    #[test]
    fn inference_python_guard_and_broker_agree_on_provider_routes() {
        use std::{
            io::Write,
            process::{Command, Stdio},
        };
        let mut cases = Vec::new();
        for (origin, provider) in [
            ("local", "workstation"),
            ("cloud", "subscription"),
            ("cloud", "another-provider"),
        ] {
            let mut p: Value = serde_json::from_str(include_str!(
                "../../../../../config/retained/hermes-profile.json"
            ))
            .unwrap();
            let model = p["model"]["default"].clone();
            p["custom_providers"][0]["extra_body"]["routing"] = json!({
                "local_only":origin=="local", "inference_origin":origin, "origin_preference":origin,
                "force_provider_id":provider, "force_model_id":model,
                "allow_forced_provider_fallback":false, "tool_use_required":true
            });
            cases.push(p.clone());
            for (field, value) in [
                ("origin_preference", json!("auto")),
                ("inference_origin", json!("auto")),
                ("allow_forced_provider_fallback", json!(true)),
                ("tool_use_required", json!(false)),
                ("force_provider_id", json!("")),
                ("force_model_id", json!("other")),
                ("local_only", json!(origin != "local")),
                ("unknown", json!(true)),
            ] {
                let mut changed = p.clone();
                changed["custom_providers"][0]["extra_body"]["routing"][field] = value;
                cases.push(changed);
            }
            p["custom_providers"][0]["base_url"] = json!("https://external.invalid/v1");
            cases.push(p);
        }
        let expected: Vec<_> = cases
            .iter()
            .map(|p| Policy::from_profile(p).is_ok())
            .collect();
        let code = concat!(include_str!("profile_guard.py"), "\nimport sys\nresults=[]\nfor config in json.load(sys.stdin):\n    try:\n        validate_profile_config(config)\n        results.append(True)\n    except (ValueError, KeyError, TypeError):\n        results.append(False)\nprint(json.dumps(results))\n");
        let mut child = Command::new("python3")
            .args(["-I", "-S", "-B", "-c", code])
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
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let actual: Vec<bool> = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.iter().filter(|accepted| **accepted).count(), 3);
    }
}
