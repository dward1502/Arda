#!/usr/bin/env bash
# sigil: REPAIR
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UNIT_DIR="${1:-$ROOT_DIR/config/systemd}"
TARGET_UNIT="$UNIT_DIR/arda-session.target"
HUD_UNIT="$UNIT_DIR/arda-hud.service"
MIRROMERE_UNIT="$UNIT_DIR/arda-mirromere.service"
MANWE_PROVIDER_CONFIG="$ROOT_DIR/config/manwe.providers.toml"

python3 - "$TARGET_UNIT" "$HUD_UNIT" "$MIRROMERE_UNIT" "$MANWE_PROVIDER_CONFIG" <<'PY'
from pathlib import Path
import sys
import tomllib

target_path = Path(sys.argv[1])
hud_path = Path(sys.argv[2])
mirromere_path = Path(sys.argv[3])
manwe_provider_path = Path(sys.argv[4])
for path in (target_path, hud_path, mirromere_path):
    if not path.is_file():
        raise SystemExit(f"missing unit: {path}")

def unit_text(path: Path) -> str:
    return "\n".join(
        line
        for line in path.read_text(encoding="utf-8").splitlines()
        if not line.lstrip().startswith(("#", ";"))
    )

target = unit_text(target_path)
hud = unit_text(hud_path)
mirromere = unit_text(mirromere_path)

def require(text: str, needle: str, label: str) -> None:
    if needle not in text:
        raise SystemExit(f"{label}: missing {needle!r}")

def forbid(text: str, needle: str, label: str) -> None:
    if needle in text:
        raise SystemExit(f"{label}: forbidden {needle!r}")

require(target, "Wants=arda.service hermes-gateway.service", "session target")
require(target, "After=network-online.target arda.service hermes-gateway.service", "session target")
forbid(target, "arda-hud.service", "session target")
forbid(target, "arda-mirromere.service", "session target")

root_runtime_path = target_path.parent / "arda.service"
if root_runtime_path.is_file():
    root_runtime = unit_text(root_runtime_path)
    require(root_runtime, "EnvironmentFile=%h/Eregion/Arda/config/.env", "Arda runtime")
    require(
        root_runtime,
        "Environment=ARDA_MANWE_PROVIDER_CONFIG=%h/Eregion/Arda/config/manwe.providers.toml",
        "Arda runtime",
    )
    require(
        root_runtime,
        "LoadCredential=arda-manwe-mutation:%h/.config/arda/credentials/hermes-gateway-capability",
        "Arda runtime",
    )
    require(root_runtime, "Environment=ARDA_OPERATOR_ID=operator:mythos", "Arda runtime")

require(hud, "Wants=arda-session.target", "HUD service")
require(hud, "After=graphical-session.target arda-session.target", "HUD service")
require(hud, "PartOf=graphical-session.target", "HUD service")
require(hud, "ExecStart=%h/.local/lib/arda/hud/arda_hud", "HUD service")
require(hud, "Environment=__NV_DISABLE_EXPLICIT_SYNC=1", "HUD service")
require(hud, "Environment=ARDA_OPERATOR_ID=operator:mythos", "HUD service")
for forbidden in (
    "target/release",
    "/Eregion/Arda",
    "launch_arda_hud.sh",
    "pnpm",
    "preview",
    "http://",
    "https://",
):
    forbid(hud, forbidden, "HUD service")

if "[Install]" in hud:
    raise SystemExit("HUD service must remain static for health-gated explicit start")

require(mirromere, "ExecStart=%h/.local/lib/arda/mirromere/arda_mirromere", "Mirromere service")
require(mirromere, "Environment=__NV_DISABLE_EXPLICIT_SYNC=1", "Mirromere service")
require(mirromere, "Environment=ARDA_OPERATOR_ID=operator:mythos", "Mirromere service")
require(mirromere, "Restart=no", "Mirromere service")
forbid(mirromere, "Wants=arda-session.target", "Mirromere service")
if "[Install]" in mirromere:
    raise SystemExit("Mirromere service must remain explicit-only")

with manwe_provider_path.open("rb") as provider_file:
    provider_config = tomllib.load(provider_file)
google = next(
    (provider for provider in provider_config.get("provider", []) if provider.get("id") == "google"),
    None,
)
if google is None:
    raise SystemExit("Manwe provider config: missing Google provider")
if google.get("probe_model") != "gemini-2.5-flash":
    raise SystemExit("Manwe provider config: Google probe must use gemini-2.5-flash")
model = next(
    (model for model in google.get("model", []) if model.get("id") == "gemini-2.5-flash"),
    None,
)
if model is None or not model.get("is_default"):
    raise SystemExit("Manwe provider config: gemini-2.5-flash must be the default")
capabilities = model.get("capabilities", {})
if not capabilities.get("tools") or not capabilities.get("structured_output"):
    raise SystemExit("Manwe provider config: gemini-2.5-flash must support tools and structured output")
PY

if command -v systemd-analyze >/dev/null 2>&1 \
  && [[ -x "$HOME/.local/lib/arda/hud/arda_hud" ]] \
  && [[ -x "$HOME/.local/lib/arda/mirromere/arda_mirromere" ]]; then
  "$ROOT_DIR/scripts/systemd_user_verify.sh" "$TARGET_UNIT" "$HUD_UNIT" "$MIRROMERE_UNIT"
fi

printf 'arda user unit verification: pass unit_dir=%s\n' "$UNIT_DIR"
