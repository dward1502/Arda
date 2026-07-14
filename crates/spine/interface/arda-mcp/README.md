---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# sigil: REPAIR
---
crate: arda-mcp
kind: integration
agent: tool-bridge
realm: communications
capabilities:
  - mcp-protocol
  - tool-surface
  - server-runtime
status: active-prototype
search_tags: [mcp, protocol, tools, server]
---

# arda-mcp

Model Context Protocol server/tool surface for Arda components.

## Purpose
Expose internal capabilities as MCP-style tools and provide server-side protocol plumbing.

## What's in this crate
- `protocol.rs`: MCP request/response structures.
- `tools.rs`: tool definitions/handlers.
- `server.rs`: MCP server runtime pieces.
- `lib.rs`: exports.
