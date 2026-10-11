---
soterion:
  sigil: "SCROLL"
  role: "bounded_qualification"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-11"
---

# Home-server measurement consumer

## Owner and boundary

Aule's existing `arda-cli metrics` command now includes `home-server`. It consumes
Beelink's existing Prometheus `job=node` series; it does not install another
exporter, scheduler or monitoring service. The existing Aule projection exporter
has local sysinfo gauges but no suitable remote PSI/swap-rate/unit collection;
the existing autopilot service-health consumer is scoped to local Arda units.
Beelink's node_exporter publishes CPU, PSI, memory, swap and filesystems, but its
systemd collector is not present in the observed metrics. Optional, fixed SSH
`systemctl list-units --state=failed --output=json` calls therefore cover system
and the SSH user's unit scopes. An empty list is not an overall health claim.

The JSON report contains raw sources, identity labels, actual scrape timestamps,
per-observation measured/stale/unavailable status, explicit units, evidence IDs,
and a deterministic summary. Model interpretation is null and actions are empty.
No inference, routing deployment, remote write, remediation or restart occurs.
SSH uses existing host keys and batch authentication. HTTP and SSH have timeouts
and 4 MiB response limits. Source failures remain unavailable, not zero values.

Freshness policy is 45 seconds, not a workload acceptance threshold. A two-minute
raw Prometheus range retains actual sample timestamps instead of query evaluation
time. Counter rates use endpoint deltas across at least ten seconds and reject
observed resets, nonfinite/negative values, malformed or ambiguous series and
future samples. CPU values are idle percentages, not inferred CPU pressure; PSI
is measured stall seconds per second. Swap page rates are distinct from occupancy
and disk-byte throughput. No sustained-capacity conclusion follows from one read.

## Invocation

From the repository root, build and invoke the existing CLI:

```sh
cargo build -p arda-aule --features http --bin arda-cli --locked
target/debug/arda-cli metrics home-server \
  --prometheus-url http://100.103.125.88:9090 \
  --node beelink --instance 100.103.125.88:9100 --ssh-target beelink
```

The output is a report even when observations are unavailable; process success
means reporting completed, not that the host is healthy. Consumers must inspect
observation statuses and timestamps. Node/instance/SSH mapping is explicitly
operator supplied, not newly registered fleet authority. This is a source-built
one-shot consumer, not an installed or scheduled service.

## Live qualification and limits

[Raw report](home-server-measurements.json) and [direct qualification](home-server-qualification.json)
retain the actual observations and comparison results. The
[read-only harness](qualify-home-server.py) reruns the CLI and compares its source
identity, all CPU series, memory/swap totals and failed system/user unit names
with independent `/proc` and `systemctl` reads. It independently recomputes every
numeric projection from retained Prometheus samples. Memory availability can
change between the two observations; its delta is recorded, not hidden.

The observed pass contains 57 measured series including 24 per-CPU idle series.
System scope reported `systemd-remount-fs.service` failed; user scope reported no
failed units. The earlier `rpm-ostree-countme.service` failure is not present in
this fresh observation and is not copied from historical model trials. Occupied
swap coexisted with zero observed swap-out rate; availability and rates remain
interval facts, not a health/pressure verdict. No model competence is claimed.

Tests: eleven pure projection regressions plus five CLI/exporter regressions pass;
364 Aule library tests with `http` pass. RED runs failed before implementation.
The fixture tests cover idle-versus-pressure, occupancy-versus-activity, omitted
failed units, stale/missing/future/reset/nonfinite observations, duplicate/wrong
identity, malformed unit output and source failures. Real HTTP fixture collection
and unavailable endpoint reporting are exercised; the latter creates no files in
its working directory. The library test warning about an unused Command import
in autopilot/workbench_executor.rs is pre-existing and outside this slice.

Independent read-only review identified boundedness and provenance gaps: SSH output
is now capped, duplicate indexing is non-quadratic with explicit row/sample limits,
and SSH-to-node identity plus CPU/filesystem completeness are explicitly unverified.
Regression coverage includes oversized noisy SSH, cardinality rejection, independent
source identity and resolution of every summary evidence ID. Independent follow-up
review found no concrete remaining blocker in these corrections. Changes are
uncommitted, with no installed
binary replacement. HUD remains inactive/MainPID=0 with no arda_hud process; Core
and Warden model-list endpoints returned HTTP 200 after qualification.

Not qualified: backup success/retention/restore, application or container health,
network exposure, sustained mixed contention, or model-generated summaries.
## Explicit inventory and liveness increment

The same command additionally reads all system/user service units and the SSH
user's rootless Podman container names/states. Repeated `--expect-system-unit`,
`--expect-user-unit` and `--expect-container` flags select running-daemon checks;
these are caller expectations, not installed policy. No expectations yields
`not_specified`, not acceptance. Fixed remote commands never interpolate selected
names. Podman output excludes environment, command arguments and mounts.

Each scope retains its bounded source, timestamp and evidence-linked checks.
`running`, `not_running`, `not_observed` and `unknown` are distinct. Missing from
an inventory does not mean uninstalled; inactive/active-exited one-shot units do
not satisfy a running-daemon check. Source failure, duplicate names, malformed
rows and stale/future observations cannot establish absence. Rootful Podman and
Docker are outside this coverage, and SSH identity remains independent.

[Retained inventory](home-inventory-measurements.json),
[qualification](home-inventory-qualification.json), and the
[read-only replay](qualify-home-inventory.py) compare all 11 observed containers
and three selected units against direct reads. The exited `minio-init` initializer
and inactive user `node_exporter.service` are negative controls, not assertions
that either must run; the distinct `node-exporter` container is running.
Three new unit tests and one new CLI fixture passed after observed RED failures;
all 367 library tests and the 11 projection/6 CLI tests pass. Independent review
of this increment found no blocking findings. Nothing was installed or restarted.

[Loopback application probes](home-application-probes.json) returned HTTP 200
from six existing endpoints: Grafana health, Prometheus readiness, MinIO readiness,
OpenWebUI health, ClickHouse ping and Langfuse health. Retained bodies preserve
what each endpoint actually reported. The [integrated report](home-application-measurements.json)
also obtains all six positive self-reports through `--application-probe` in the
existing command, e.g. `--application-probe grafana=http://100.103.125.88:3000`.
Supported kinds are grafana, prometheus, minio, openwebui, clickhouse and langfuse;
paths are fixed per kind. Up to 16 explicit probes use four-second/16-KiB bounds,
no credentials/proxies/redirects, and application-specific HTTP/body assertions.
Stale, malformed or failed sources cannot become a current positive self-report.
Raw response text is evidence, never model instructions or executable content.
Four application regressions and seven CLI tests pass alongside eleven existing
projection tests; all 371 library tests pass. Independent review found no concrete
blockers and validated all six retained observations. This is endpoint self-report
integration, not representative workload acceptance or backup verification.
[Backup timer/tool discovery](home-backup-discovery.json) found restic, borg and
rclone installed but did not establish a repository or retained backup/restore
success. The operator subsequently confirmed backup setup is outstanding. No
repository was initialized, backup run, or restore attempted.

Next backup gate: select an independent destination and protected datasets, then
approve application-consistent backup, retention and isolated restore testing.
This is setup work requiring separate approval, not missing monitoring evidence
for an assumed existing backup. Sustained workload and model-summary quality
remain open. Retained reports must be re-evaluated against the current clock
before being treated as current observations.
