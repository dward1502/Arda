#!/usr/bin/env python3
"""Render/verify explicitly selected user units, without reading secret values.

The composition installs service wiring only: it neither provisions stores nor
starts services, configures network isolation, or declares runtime readiness.
"""
import argparse
import json
import os
from pathlib import Path
import re

SOURCE = Path(__file__).resolve().parents[1] / "config/systemd"


def selected_path(value, *, private=False, directory=False, executable=False):
    if not isinstance(value, str) or not value.startswith("/"):
        raise ValueError("composition paths must be absolute")
    # Deliberately bounded systemd syntax; never interpret specifiers or shell text.
    if any(ord(c) < 32 or c in '%$\\"' for c in value):
        raise ValueError("unsupported character in composition path")
    path = Path(value).resolve(strict=True)
    if any(ord(c) < 32 or c in '%$\\"' for c in str(path)):
        raise ValueError("unsupported character in resolved composition path")
    if private and not path.is_relative_to(Path.home().resolve()):
        raise ValueError("private configuration must belong to the selected home")
    if directory:
        if not path.is_dir():
            raise ValueError("release root must be a directory")
    elif not path.is_file():
        raise ValueError("composition input must be a regular file")
    if executable and not os.access(path, os.X_OK):
        raise ValueError("root executable is not executable")
    return str(path)


def render(composition):
    config = json.loads(Path(composition).read_text())
    required = {"release_root", "root_executable", "operator_id", "environment_file", "provider_config", "credentials"}
    if not isinstance(config, dict) or set(config) != required:
        raise ValueError("composition must contain exactly: " + ", ".join(sorted(required)))
    root = selected_path(config["release_root"], directory=True)
    if not (Path(root) / "services.toml").is_file():
        raise ValueError("release root must contain services.toml")
    binary = selected_path(config["root_executable"], executable=True)
    environment = selected_path(config["environment_file"], private=True)
    provider = selected_path(config["provider_config"], private=True)
    operator = config["operator_id"]
    if not isinstance(operator, str) or not re.fullmatch(r"operator:[A-Za-z0-9_.-]+", operator):
        raise ValueError("invalid explicit operator identity")
    credentials = config["credentials"]
    if not isinstance(credentials, dict):
        raise ValueError("credentials must be an explicit name/path map")
    credential_lines = []
    for name, value in sorted(credentials.items()):
        if not re.fullmatch(r"[A-Za-z0-9_.-]+", name):
            raise ValueError("invalid credential name")
        credential_lines.append(f'LoadCredential={name}:{selected_path(value, private=True)}')
    # CLI identity wins over any unrelated inherited environment-file setting.
    runtime = "\n".join([
        "[Unit]", "Description=Arda explicit account-local root runtime",
        "After=network-online.target", "Wants=network-online.target", "",
        "[Service]", "Type=simple", f'WorkingDirectory={root}',
        f'EnvironmentFile={environment}', "Environment=RUST_LOG=info",
        f'Environment="ARDA_ROOT={root}"',
        f'Environment="ARDA_MANWE_PROVIDER_CONFIG={provider}"',
        f"Environment=ARDA_OPERATOR_ID={operator}",
        *credential_lines,
        f'ExecStart="{binary}" --no-ui --operator-id {operator}',
        "Restart=on-failure", "RestartSec=3", "",
        "[Install]", "WantedBy=default.target", "Alias=arda-manwe.service", "",
    ])
    units = {"arda.service": runtime}
    for name in ("arda-session.target", "arda-hud.service", "arda-mirromere.service",
                 "hermes-gateway.service.d/arda-watchdog.conf"):
        text = (SOURCE / name).read_text()
        units[name] = text.replace("Environment=ARDA_OPERATOR_ID=operator:mythos",
                                   f"Environment=ARDA_OPERATOR_ID={operator}")
    return units


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("composition")
    parser.add_argument("unit_dir", type=Path)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    try:
        units = render(args.composition)  # Validate every input before any write.
        for name, text in units.items():
            path = args.unit_dir / name
            if args.verify:
                if not path.is_file() or path.read_text() != text:
                    raise ValueError(f"unit does not match selected composition: {name}")
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text)
    except (OSError, ValueError) as error:
        parser.exit(1, f"root composition: {error}\n")


if __name__ == "__main__":
    main()
