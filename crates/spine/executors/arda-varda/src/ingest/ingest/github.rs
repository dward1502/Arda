// sigil: REPAIR
//
// GitHub repo + file metadata enrichment. Mirrors the scholarly.rs pattern:
// for a `SourceType::GithubRepo` or `GithubFile` URL, hit the GitHub REST API
// and populate structured fields so the shallow ingest record stops being a
// bookmark and becomes a real quick reference.
//
// Auth: reads `ARDA_GITHUB_TOKEN` (preferred) or `GITHUB_TOKEN` from env.
// Without a token: 60 req/hr public limit; with: 5000 req/hr.

use arda_core::error::Result;
use serde::{Deserialize, Serialize};

use super::{athena_error, routing, GithubMetadata};

const USER_AGENT: &str = "annunimas-athena/0.1 (+https://github.com/annunimas)";
const README_CHAR_LIMIT: usize = 16_384;

#[derive(Debug, Clone)]
pub(super) struct RepoCoords {
    pub owner: String,
    pub repo: String,
    pub file_path: Option<String>,
    pub ref_name: Option<String>,
}

pub(super) fn parse_repo_coords(url: &str) -> Option<RepoCoords> {
    let after_scheme = url.split("://").nth(1)?;
    let path = after_scheme.split_once('/').map(|(_, rest)| rest)?;
    let mut parts = path
        .split('/')
        .filter(|segment| {
            !segment.is_empty() && !segment.starts_with('?') && !segment.starts_with('#')
        })
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts.remove(0).to_string();
    let repo = parts
        .remove(0)
        .trim_end_matches(".git")
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    let (ref_name, file_path) =
        if parts.first().copied() == Some("blob") || parts.first().copied() == Some("tree") {
            parts.remove(0);
            let ref_name = if parts.is_empty() {
                None
            } else {
                Some(parts.remove(0).to_string())
            };
            let file = if parts.is_empty() {
                None
            } else {
                Some(parts.join("/"))
            };
            (ref_name, file)
        } else if parts.first().copied() == Some("raw") {
            // raw.githubusercontent.com paths get a different host upstream, but
            // github.com/.../raw/<ref>/<path> also appears occasionally.
            parts.remove(0);
            let ref_name = if parts.is_empty() {
                None
            } else {
                Some(parts.remove(0).to_string())
            };
            let file = if parts.is_empty() {
                None
            } else {
                Some(parts.join("/"))
            };
            (ref_name, file)
        } else {
            (None, None)
        };

    Some(RepoCoords {
        owner,
        repo,
        file_path,
        ref_name,
    })
}

pub(super) fn fetch_github_metadata(url: &str) -> Option<GithubMetadata> {
    let Some(coords) = parse_repo_coords(url) else {
        tracing::debug!(url = %url, "github fetch skipped: could not parse owner/repo");
        return None;
    };
    if offline_forced() {
        tracing::debug!(url = %url, "github fetch skipped: offline forced");
        return None;
    }
    match routing::run_async_for_sync(fetch_repo_metadata(coords, url.to_string())) {
        Ok(meta) => Some(meta),
        Err(e) => {
            tracing::warn!(url = %url, error = %e, "github fetch failed");
            None
        }
    }
}

fn offline_forced() -> bool {
    std::env::var("ARDA_ATHENA_FORCE_OFFLINE_GITHUB_METADATA")
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn github_token() -> Option<String> {
    std::env::var("ARDA_GITHUB_TOKEN")
        .ok()
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| athena_error(format!("github client build failed: {e}")))
}

fn apply_auth(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let mut req = req
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = github_token() {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    req
}

async fn fetch_repo_metadata(coords: RepoCoords, source_url: String) -> Result<GithubMetadata> {
    let client = build_client()?;
    let repo_url = format!(
        "https://api.github.com/repos/{}/{}",
        coords.owner, coords.repo
    );
    let repo: RepoResponse = apply_auth(client.get(&repo_url))
        .send()
        .await
        .map_err(|e| athena_error(format!("github repo fetch failed: {e}")))?
        .error_for_status()
        .map_err(|e| athena_error(format!("github repo response invalid: {e}")))?
        .json()
        .await
        .map_err(|e| athena_error(format!("github repo decode failed: {e}")))?;

    let readme = fetch_readme(&client, &coords.owner, &coords.repo)
        .await
        .ok()
        .map(|text| truncate_chars(&text, README_CHAR_LIMIT));

    let manifest = fetch_manifest(
        &client,
        &coords.owner,
        &coords.repo,
        repo.default_branch.as_deref(),
    )
    .await
    .ok();

    let (key_dependencies, manifest_kind) = manifest
        .as_ref()
        .map(|m| (m.dependencies.clone(), Some(m.kind.clone())))
        .unwrap_or_default();

    let topics = repo.topics.clone().unwrap_or_default();
    let license = repo
        .license
        .as_ref()
        .and_then(|l| l.spdx_id.clone().or_else(|| l.name.clone()))
        .filter(|s| s != "NOASSERTION");

    Ok(GithubMetadata {
        owner: coords.owner,
        repo: coords.repo,
        full_name: repo.full_name,
        description: repo.description,
        primary_language: repo.language,
        license,
        default_branch: repo.default_branch,
        stargazers_count: repo.stargazers_count,
        forks_count: repo.forks_count,
        open_issues_count: repo.open_issues_count,
        pushed_at: repo.pushed_at,
        topics,
        readme_excerpt: readme,
        manifest_kind,
        key_dependencies,
        file_path: coords.file_path,
        ref_name: coords.ref_name,
        source_url,
    })
}

async fn fetch_readme(client: &reqwest::Client, owner: &str, repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/readme");
    let response = apply_auth(
        client
            .get(&url)
            .header("Accept", "application/vnd.github.raw"),
    )
    .send()
    .await
    .map_err(|e| athena_error(format!("github readme fetch failed: {e}")))?;
    if !response.status().is_success() {
        return Err(athena_error(format!(
            "github readme status {}",
            response.status()
        )));
    }
    response
        .text()
        .await
        .map_err(|e| athena_error(format!("github readme decode failed: {e}")))
}

#[derive(Debug, Clone)]
struct ManifestParse {
    kind: String,
    dependencies: Vec<String>,
}

async fn fetch_manifest(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    default_branch: Option<&str>,
) -> Result<ManifestParse> {
    let candidates = [
        ("Cargo.toml", "cargo"),
        ("package.json", "npm"),
        ("pyproject.toml", "pyproject"),
        ("requirements.txt", "requirements"),
        ("go.mod", "gomod"),
    ];
    let mut cargo_workspace_members: Vec<String> = Vec::new();
    for (path, kind) in candidates {
        if let Ok(text) = fetch_raw_file(client, owner, repo, path, default_branch).await {
            if kind == "cargo" {
                cargo_workspace_members = parse_cargo_workspace_members(&text);
            }
            if let Some(deps) = parse_manifest_deps(kind, &text) {
                if !deps.is_empty() {
                    return Ok(ManifestParse {
                        kind: kind.to_string(),
                        dependencies: deps,
                    });
                }
            }
        }
    }

    // Cargo workspace: root has no deps, descend into members.
    for member in cargo_workspace_members.iter().take(8) {
        let member_path = format!("{member}/Cargo.toml");
        if let Ok(text) = fetch_raw_file(client, owner, repo, &member_path, default_branch).await {
            let deps = parse_cargo_deps(&text);
            if !deps.is_empty() {
                return Ok(ManifestParse {
                    kind: "cargo".to_string(),
                    dependencies: deps,
                });
            }
        }
    }

    // Cargo workspace with glob members (e.g. "crates/*"): list directory and try first hits.
    for glob_member in cargo_workspace_members
        .iter()
        .filter(|m| m.ends_with("/*"))
        .take(2)
    {
        let dir = glob_member.trim_end_matches("/*");
        if let Ok(entries) = list_contents(client, owner, repo, dir, default_branch).await {
            for entry in entries.iter().take(8) {
                let member_path = format!("{dir}/{entry}/Cargo.toml");
                if let Ok(text) =
                    fetch_raw_file(client, owner, repo, &member_path, default_branch).await
                {
                    let deps = parse_cargo_deps(&text);
                    if !deps.is_empty() {
                        return Ok(ManifestParse {
                            kind: "cargo".to_string(),
                            dependencies: deps,
                        });
                    }
                }
            }
        }
    }

    Err(athena_error("no parseable manifest found"))
}

async fn list_contents(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    path: &str,
    default_branch: Option<&str>,
) -> Result<Vec<String>> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}");
    let mut req = apply_auth(client.get(&url));
    if let Some(branch) = default_branch {
        req = req.query(&[("ref", branch)]);
    }
    let response = req
        .send()
        .await
        .map_err(|e| athena_error(format!("github contents list failed: {e}")))?
        .error_for_status()
        .map_err(|e| athena_error(format!("github contents list status: {e}")))?;
    let entries: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| athena_error(format!("github contents decode failed: {e}")))?;
    Ok(entries
        .into_iter()
        .filter(|e| e.get("type").and_then(|v| v.as_str()) == Some("dir"))
        .filter_map(|e| {
            e.get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect())
}

pub(super) fn parse_cargo_workspace_members(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_workspace = false;
    let mut buffer = String::new();
    let mut in_members = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_workspace = line == "[workspace]";
            in_members = false;
            continue;
        }
        if !in_workspace {
            continue;
        }
        if line.starts_with("members") || in_members {
            if !in_members {
                in_members = true;
                buffer.clear();
                if let Some((_, rest)) = line.split_once('=') {
                    buffer.push_str(rest);
                }
            } else {
                buffer.push(' ');
                buffer.push_str(line);
            }
            if buffer.contains(']') {
                let inner = buffer
                    .trim()
                    .trim_start_matches('=')
                    .trim()
                    .trim_start_matches('[')
                    .split(']')
                    .next()
                    .unwrap_or("");
                for token in inner.split(',') {
                    let name = token.trim().trim_matches(|c: char| c == '"' || c == '\'');
                    if !name.is_empty() {
                        out.push(name.to_string());
                    }
                }
                in_members = false;
                buffer.clear();
            }
        }
    }
    out
}

async fn fetch_raw_file(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    path: &str,
    default_branch: Option<&str>,
) -> Result<String> {
    let url = format!("https://api.github.com/repos/{owner}/{repo}/contents/{path}");
    let mut req = apply_auth(
        client
            .get(&url)
            .header("Accept", "application/vnd.github.raw"),
    );
    if let Some(branch) = default_branch {
        req = req.query(&[("ref", branch)]);
    }
    let response = req
        .send()
        .await
        .map_err(|e| athena_error(format!("github raw {path} fetch failed: {e}")))?;
    if !response.status().is_success() {
        return Err(athena_error(format!(
            "github raw {path} status {}",
            response.status()
        )));
    }
    response
        .text()
        .await
        .map_err(|e| athena_error(format!("github raw {path} decode failed: {e}")))
}

pub(super) fn parse_manifest_deps(kind: &str, text: &str) -> Option<Vec<String>> {
    match kind {
        "cargo" => Some(parse_cargo_deps(text)),
        "npm" => Some(parse_package_json_deps(text)),
        "pyproject" => Some(parse_pyproject_deps(text)),
        "requirements" => Some(parse_requirements_deps(text)),
        "gomod" => Some(parse_gomod_deps(text)),
        _ => None,
    }
}

fn parse_cargo_deps(text: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            let section = line.trim_matches(|c| c == '[' || c == ']').trim();
            in_deps = matches!(
                section,
                "dependencies"
                    | "dev-dependencies"
                    | "build-dependencies"
                    | "workspace.dependencies"
            );
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim().trim_matches('"');
            if !name.is_empty() {
                deps.push(name.to_string());
            }
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

fn parse_package_json_deps(text: &str) -> Vec<String> {
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for key in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(obj) = value.get(key).and_then(|v| v.as_object()) {
            for name in obj.keys() {
                deps.push(name.clone());
            }
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

fn parse_pyproject_deps(text: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_block = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            in_block = matches!(
                line,
                "[project]"
                    | "[tool.poetry.dependencies]"
                    | "[tool.poetry.dev-dependencies]"
                    | "[tool.poetry.group.dev.dependencies]"
            );
            continue;
        }
        if !in_block || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("dependencies") {
            // PEP 621 list form: dependencies = ["foo>=1", "bar"]
            if let Some(arr_start) = rest.find('[') {
                let arr = &rest[arr_start + 1..];
                for chunk in arr.split(',') {
                    let name = chunk
                        .trim()
                        .trim_matches(|c: char| c == '"' || c == '\'' || c == ']')
                        .split(['>', '<', '=', '!', '~', ';', '['])
                        .next()
                        .unwrap_or("")
                        .trim();
                    if !name.is_empty() {
                        deps.push(name.to_string());
                    }
                }
            }
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim().trim_matches('"');
            if !name.is_empty() && !name.contains(' ') {
                deps.push(name.to_string());
            }
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

fn parse_requirements_deps(text: &str) -> Vec<String> {
    let mut deps = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            continue;
        }
        let name = line
            .split(['>', '<', '=', '!', '~', ';', '['])
            .next()
            .unwrap_or("")
            .trim();
        if !name.is_empty() {
            deps.push(name.to_string());
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

fn parse_gomod_deps(text: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_require = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("require (") {
            in_require = true;
            continue;
        }
        if in_require && line == ")" {
            in_require = false;
            continue;
        }
        if let Some(rest) = line.strip_prefix("require ") {
            if let Some(name) = rest.split_whitespace().next() {
                deps.push(name.to_string());
            }
            continue;
        }
        if in_require && !line.is_empty() && !line.starts_with("//") {
            if let Some(name) = line.split_whitespace().next() {
                deps.push(name.to_string());
            }
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

fn truncate_chars(input: &str, limit: usize) -> String {
    input.chars().take(limit).collect()
}

#[derive(Debug, Deserialize, Serialize)]
struct RepoResponse {
    full_name: String,
    description: Option<String>,
    language: Option<String>,
    default_branch: Option<String>,
    stargazers_count: Option<u64>,
    forks_count: Option<u64>,
    open_issues_count: Option<u64>,
    pushed_at: Option<String>,
    topics: Option<Vec<String>>,
    license: Option<LicenseResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LicenseResponse {
    spdx_id: Option<String>,
    name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_repo_url() {
        let coords = parse_repo_coords("https://github.com/AdamStrojek/rust-agentai").unwrap();
        assert_eq!(coords.owner, "AdamStrojek");
        assert_eq!(coords.repo, "rust-agentai");
        assert!(coords.file_path.is_none());
        assert!(coords.ref_name.is_none());
    }

    #[test]
    fn parses_blob_file_url() {
        let coords =
            parse_repo_coords("https://github.com/owner/repo/blob/main/src/lib.rs").unwrap();
        assert_eq!(coords.repo, "repo");
        assert_eq!(coords.ref_name.as_deref(), Some("main"));
        assert_eq!(coords.file_path.as_deref(), Some("src/lib.rs"));
    }

    #[test]
    fn strips_dot_git_suffix() {
        let coords = parse_repo_coords("https://github.com/owner/repo.git").unwrap();
        assert_eq!(coords.repo, "repo");
    }

    #[test]
    fn rejects_non_repo_urls() {
        assert!(parse_repo_coords("https://github.com/owner").is_none());
    }

    #[test]
    fn parses_cargo_deps() {
        let toml = r#"
[package]
name = "x"

[dependencies]
serde = "1"
tokio = { version = "1", features = ["full"] }

[dev-dependencies]
proptest = "1"
"#;
        let deps = parse_cargo_deps(toml);
        assert!(deps.contains(&"serde".to_string()));
        assert!(deps.contains(&"tokio".to_string()));
        assert!(deps.contains(&"proptest".to_string()));
    }

    #[test]
    fn parses_package_json_deps() {
        let json =
            r#"{"dependencies":{"react":"^18","next":"^14"},"devDependencies":{"jest":"^29"}}"#;
        let deps = parse_package_json_deps(json);
        assert!(deps.contains(&"react".to_string()));
        assert!(deps.contains(&"next".to_string()));
        assert!(deps.contains(&"jest".to_string()));
    }

    #[test]
    fn parses_workspace_members_inline() {
        let toml = "[workspace]\nresolver = \"3\"\nmembers = [\"crates/*\", \"apps/cli\"]\n";
        let members = parse_cargo_workspace_members(toml);
        assert_eq!(
            members,
            vec!["crates/*".to_string(), "apps/cli".to_string()]
        );
    }

    #[test]
    fn parses_workspace_members_multiline() {
        let toml = "[workspace]\nmembers = [\n  \"crates/core\",\n  \"crates/cli\",\n]\n";
        let members = parse_cargo_workspace_members(toml);
        assert!(members.contains(&"crates/core".to_string()));
        assert!(members.contains(&"crates/cli".to_string()));
    }

    #[test]
    fn parses_requirements_deps() {
        let text = "# top\nrequests>=2.0\nnumpy==1.24\n";
        let deps = parse_requirements_deps(text);
        assert_eq!(deps, vec!["numpy".to_string(), "requests".to_string()]);
    }
}
