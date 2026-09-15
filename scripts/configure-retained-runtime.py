#!/usr/bin/env python3
"""Generate a private retained runtime config; never copy ambient credentials.

Default is a dry run. --apply exclusively creates missing files and refuses to
replace differing operator files. This does not initialize a keeper or services.
"""
import argparse
from contextlib import contextmanager
import json
import os
from pathlib import Path
import stat
import subprocess
import tomllib
import uuid

REPO = Path(__file__).resolve().parents[1]


def canonical(raw, directory=False, must_exist=True):
    path = Path(raw)
    if not path.is_absolute() or str(path) != str(raw):
        raise ValueError("paths must use canonical absolute spelling")
    if path.resolve(strict=must_exist) != path:
        raise ValueError("symlinked paths are not accepted")
    if must_exist and (path.is_dir() if directory else path.is_file()) is False:
        raise ValueError("path has the wrong type")
    return path


@contextmanager
def pinned_directory(path, create):
    """Walk from / using openat; never follow replaced parent symlinks."""
    fd = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        for part in path.parts[1:]:
            if create:
                try:
                    os.mkdir(part, mode=0o700, dir_fd=fd)
                except FileExistsError:
                    pass
            try:
                child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC, dir_fd=fd)
            except FileNotFoundError:
                if create:
                    raise
                yield None
                return
            os.close(fd)
            fd = child
        info = os.fstat(fd)
        if info.st_uid != os.geteuid() or info.st_mode & 0o077:
            raise ValueError("output/state directories must be private and owned")
        yield fd
    finally:
        os.close(fd)


def matches_existing(directory, name, content):
    try:
        fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=directory)
    except FileNotFoundError:
        return False
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.geteuid()
                or info.st_mode & 0o077 or info.st_nlink != 1):
            raise ValueError("existing output must be a private owned regular file")
        if stream.read(len(content) + 1) != content:
            raise ValueError("refusing to replace a differing operator file")
        current = os.stat(name, dir_fd=directory, follow_symlinks=False)
        if (current.st_dev, current.st_ino) != (info.st_dev, info.st_ino):
            raise ValueError("output changed during validation")
    return True


def publish(directory, name, content):
    temporary = ".retained-" + uuid.uuid4().hex
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=directory)
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        try:
            # Publish complete bytes without overwriting a concurrent creator.
            os.link(temporary, name, src_dir_fd=directory, dst_dir_fd=directory, follow_symlinks=False)
        except FileExistsError:
            if not matches_existing(directory, name, content):
                raise ValueError("output disappeared during publication")
    finally:
        os.unlink(temporary, dir_fd=directory)
        os.fsync(directory)


def adapter_config(legacy):
    # Keep validator limits and role toolsets unchanged. Only retained execution
    # uses this config; ordinary execution/replay continue using the original.
    if legacy.get("working_directory") is not None:
        raise ValueError("retained execution requires project-root working directory")
    if legacy.get("schema_version") != "arda.hermes-adapter.v1":
        raise ValueError("unsupported ordinary adapter schema")
    version = legacy.get("adapter_version")
    if not isinstance(version, str) or not version.strip():
        raise ValueError("missing adapter version")
    lines = ['schema_version = "arda.hermes-adapter.v1"',
             f'adapter_version = {json.dumps(version)}',
             'executable = "hermes"', 'inherit_environment = []']
    for key in ("max_timeout_ms", "cancellation_grace_ms", "max_prompt_bytes", "max_output_bytes", "max_turns"):
        value = legacy.get(key)
        if type(value) is not int or value <= 0:
            raise ValueError("invalid or missing adapter budget")
        lines.append(f"{key} = {value}")
    if legacy["max_timeout_ms"] > 300_000 or legacy["max_output_bytes"] > 1_048_576:
        raise ValueError("adapter budget exceeds retained transport limits")
    roles = {"read_only", "verify", "execute_with_approval", "human_approval", "compensate_with_approval"}
    if not isinstance(legacy.get("toolsets"), dict) or set(legacy["toolsets"]) != roles:
        raise ValueError("missing or unknown adapter role")
    if "toolsets" in legacy:
        lines.append("\n[toolsets]")
        for role, values in legacy["toolsets"].items():
            if role not in roles:
                raise ValueError("unknown adapter role")
            if not isinstance(values, list) or any(v not in {"file", "terminal"} for v in values):
                raise ValueError("retained runtime supports file/terminal toolsets only")
            lines.append(f"{role} = {json.dumps(values)}")
    text = "\n".join(lines) + "\n"
    tomllib.loads(text)
    return text.encode()


def generate(args):
    output = canonical(args.output_root, directory=True, must_exist=False)
    sessions = canonical(args.session_base, directory=True, must_exist=False)
    source = canonical(args.hermes_source, directory=True)
    python = canonical(args.python)
    if not python.is_relative_to(Path("/usr/bin")):
        raise ValueError("interpreter must be provided by captured /usr/bin")
    metadata = subprocess.run(
        [str(python), "-I", "-S", "-c", "import json,sys,sysconfig; system=sysconfig.get_paths(vars={'base':'/usr','platbase':'/usr'}); print(json.dumps({'version':list(sys.version_info[:2]),'sites':[sysconfig.get_path('purelib'),sysconfig.get_path('platlib'),system['purelib'],system['platlib']]}))"],
        check=True, capture_output=True, text=True, timeout=10,
    )
    metadata = json.loads(metadata.stdout)
    version = ".".join(map(str, metadata["version"]))
    site = canonical(args.user_site or str(Path.home() / f".local/lib/python{version}/site-packages"), directory=True)
    imports = [str(source), str(site)]
    for discovered in metadata["sites"]:
        if not Path(discovered).exists():
            continue
        path = canonical(str(Path(discovered).resolve(strict=True)), directory=True)
        if not path.is_relative_to(Path("/usr")):
            raise ValueError("system Python dependencies must be under the captured /usr")
        if str(path) not in imports:
            imports.append(str(path))
    for root in (source, site):
        for name in (".env", ".op.env", "auth.json", "config.yaml"):
            if (root / name).exists() or (root / name).is_symlink():
                raise ValueError("runtime source contains ambient authority inputs")
    if output.is_relative_to(sessions) or sessions.is_relative_to(output):
        raise ValueError("profile inputs and session state must be separate")
    profile = output / "config.yaml"
    home = "/hermes/profiles/retained"
    def grant(key, src, destination, kind, access, role):
        return dict(id=key, source=str(src), destination=destination, kind=kind, access=access, role=role)
    policy = dict(version=1,
        entrypoint=dict(interpreter=str(python), ordered_import_roots=imports),
        grants=[
            grant("system", "/usr", "/usr", "directory", "read_only", "runtime"),
            grant("hermes", source, str(source), "directory", "read_only", "runtime"),
            grant("user_site", site, str(site), "directory", "read_only", "runtime"),
            grant("profile", profile, home + "/config.yaml", "file", "read_only", "profile_input"),
            grant("state", sessions, home, "directory", "read_write", "session_state"),
        ], fixed_environment=dict(HOME=home, HERMES_HOME=home, PATH="/usr/bin:/bin"))
    legacy = tomllib.loads(canonical(args.legacy_adapter).read_text())
    files = {
        output / "config.yaml": (REPO / "config/retained/hermes-profile.json").read_bytes(),
        output / "runtime-policy.json": (json.dumps(policy, indent=2) + "\n").encode(),
        output / "hermes-adapter.toml": adapter_config(legacy),
    }
    with pinned_directory(output, create=args.apply) as directory:
        with pinned_directory(sessions, create=args.apply):
            if directory is not None:
                # Preflight conflicts; each publication also validates racing files.
                # This is per-file atomic, not an all-or-nothing batch transaction.
                for path, content in files.items():
                    matches_existing(directory, path.name, content)
                if args.apply:
                    for path, content in files.items():
                        publish(directory, path.name, content)
    return {"applied": args.apply, "output_root": str(output), "session_base": str(sessions),
            "files": [str(path) for path in files], "interpreter": str(python),
            "note": "schema/import provisioning only; no services initialized or started"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-root", default=str(Path.home() / ".config/arda/retained"))
    parser.add_argument("--session-base", default=str(Path.home() / ".local/share/arda/retained/sessions"))
    parser.add_argument("--hermes-source", default=str(Path.home() / ".hermes/hermes-agent"))
    parser.add_argument("--python", default=str(Path("/usr/bin/python3").resolve()))
    parser.add_argument("--user-site")
    parser.add_argument("--legacy-adapter", required=True)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    print(json.dumps(generate(args), indent=2))


if __name__ == "__main__":
    main()
