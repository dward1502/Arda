"""Source-only reads at the owned execution boundary, never generic file dispatch."""
def enforce_readonly_tools(model_tools, registry, root=None):
    import fnmatch
    import json
    import os
    from pathlib import Path
    import stat

    root = Path(root or os.getcwd()).resolve(strict=True)
    allowed = frozenset(("read_file", "search_files"))
    definitions = model_tools.get_tool_definitions
    source_dirs = frozenset(("src", "crates", "adapters", "apps", "sdk", "tests", "spec", "scripts", "docs", "examples", "include", "lib"))
    source_suffixes = frozenset((".rs", ".py", ".js", ".ts", ".tsx", ".jsx", ".md", ".toml", ".json", ".yaml", ".yml", ".sh", ".html", ".css", ".c", ".h", ".cpp", ".txt"))
    root_files = frozenset(("README.md", "AGENTS.md", "Cargo.toml", "package.json", "pyproject.toml", "Makefile", "CMakeLists.txt", "LICENSE", "LICENSE.md"))
    forbidden = frozenset(("target", "node_modules", "data", "state", "dist", "vendor", "__pycache__"))

    def source_path(raw, directory=False):
        path = Path(raw)
        path = path if path.is_absolute() else root / path
        relative = path.relative_to(root)
        if ".." in relative.parts:
            raise ValueError("parent traversal denied")
        current = root
        for part in relative.parts:
            current /= part
            if current.is_symlink() or part.startswith(".") or part in forbidden:
                raise ValueError("non-source path denied")
            if part.lower() in ("auth.json", "config.yaml", "config.yml", "config.json", "id_rsa", "id_ed25519"):
                raise ValueError("authentication/configuration container denied")
            if any(word in part.lower() for word in ("secret", "credential", "password", "private-key")):
                raise ValueError("sensitive path denied")
        if path.resolve(strict=True) != path:
            raise ValueError("noncanonical source path denied")
        if len(relative.parts) > 1 and relative.parts[0] not in source_dirs:
            raise ValueError("source subtree not allowed")
        if directory and path.is_dir():
            if relative.parts and relative.parts[0] not in source_dirs:
                raise ValueError("source subtree not allowed")
            return path
        if len(relative.parts) == 1 and path.name not in root_files:
            raise ValueError("root file not explicitly allowed")
        if not path.is_file() or (path.suffix.lower() not in source_suffixes and path.name not in root_files):
            raise ValueError("source file type not allowed")
        if path.stat().st_nlink != 1:
            raise ValueError("multiply linked source denied")
        return path

    def read_source(path):
        # Read text only, without converter subprocesses, fallback paths or links.
        parent = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
        try:
            parts = path.relative_to(root).parts
            for part in parts[:-1]:
                child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent)
                os.close(parent)
                parent = child
            fd = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
        finally:
            os.close(parent)
        with os.fdopen(fd, "rb") as handle:
            metadata = os.fstat(handle.fileno())
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1 or metadata.st_size > 2_097_152:
                raise ValueError("source file exceeds bounded text scope")
            return handle.read(2_097_153).decode("utf-8")

    def source_files(path):
        if path.is_file():
            yield source_path(str(path))
            return
        for parent, directories, files in os.walk(path, followlinks=False):
            kept = []
            for name in sorted(directories):
                try:
                    source_path(str(Path(parent) / name), directory=True)
                    kept.append(name)
                except (OSError, ValueError):
                    pass
            directories[:] = kept
            for name in sorted(files):
                try:
                    yield source_path(str(Path(parent) / name))
                except (OSError, ValueError):
                    pass

    def readonly_definitions(*args, **kwargs):
        result = []
        for tool in definitions(*args, **kwargs):
            if tool.get("function", {}).get("name") not in allowed:
                continue
            tool = json.loads(json.dumps(tool))
            tool["function"]["description"] = (
                "Read approved source text only within the project root. Hidden files, "
                "credentials, runtime data, build outputs and symlinks are denied. "
                "Search content uses literal matching (not regex); filename search uses globs.")
            supported = ({"path", "offset", "limit"} if tool["function"]["name"] == "read_file"
                         else {"pattern", "path", "target", "file_glob", "limit", "offset"})
            parameters = tool["function"].get("parameters", {})
            parameters["properties"] = {key: value for key, value in parameters.get("properties", {}).items() if key in supported}
            parameters["additionalProperties"] = False
            tool["function"]["parameters"] = parameters
            result.append(tool)
        return result

    def readonly_dispatch(name, args, **kwargs):
        if name not in allowed:
            return '{"error":"tool denied by read-only runtime authority"}'
        try:
            if not isinstance(args, dict):
                raise ValueError("structured arguments required")
            if name == "read_file":
                if set(args) - {"path", "offset", "limit"}:
                    raise ValueError("unsupported source-read argument")
                path = source_path(args["path"])
                lines = read_source(path).splitlines()
                offset, limit = int(args.get("offset", 1)), int(args.get("limit", 200))
                if offset < 1 or not 1 <= limit <= 2000:
                    raise ValueError("source-read bounds invalid")
                content = "\n".join(f"{n + 1}|{line}" for n, line in enumerate(lines)
                                    if offset - 1 <= n < offset - 1 + limit)
                return json.dumps({"content": content[:100_000], "total_lines": len(lines),
                                   "path": str(path), "source_only": True})
            if set(args) - {"pattern", "path", "target", "file_glob", "limit", "offset"}:
                raise ValueError("unsupported source-search argument")
            target = args.get("target", "content")
            if target not in ("content", "files"):
                raise ValueError("unsupported search target")
            path = source_path(args.get("path", "."), directory=True)
            limit, offset = int(args.get("limit", 50)), int(args.get("offset", 0))
            if not 1 <= limit <= 200 or offset < 0:
                raise ValueError("source-search bounds invalid")
            pattern = str(args["pattern"])
            # Literal content matching is intentional: no unbounded regex execution.
            matches = []
            visited = 0
            for candidate in source_files(path):
                visited += 1
                if visited > 20_000:
                    break
                relative = str(candidate.relative_to(root))
                glob = args.get("file_glob")
                if glob and not (fnmatch.fnmatch(relative, glob) or fnmatch.fnmatch(candidate.name, glob)):
                    continue
                if target == "files":
                    if fnmatch.fnmatch(relative, pattern) or fnmatch.fnmatch(candidate.name, pattern):
                        matches.append(relative)
                else:
                    try:
                        for number, line in enumerate(read_source(candidate).splitlines(), 1):
                            if pattern in line:
                                matches.append({"path": relative, "line": number, "content": line[:1000]})
                                if len(matches) > offset + limit:
                                    break
                    except (OSError, ValueError, UnicodeError):
                        continue
                if len(matches) > offset + limit:
                    break
            return json.dumps({"matches": matches[offset:offset + limit],
                               "truncated": len(matches) > offset + limit or visited > 20_000,
                               "source_only": True, "content_matching": "literal"})
        except (OSError, ValueError, KeyError, TypeError) as error:
            return json.dumps({"error": f"source-only scope denied: {type(error).__name__}"})

    model_tools.get_tool_definitions = readonly_definitions
    registry.dispatch = readonly_dispatch
