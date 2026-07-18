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

# sigil: SCROLL
# Edge Connectivity Surface

Reference files for ATHENA/HERMES edge connectivity and model placement.

## Files
- `targets.example.toml` - device inventory template (Tailscale + local fallbacks)
- `model_profiles.toml` - role-based model assignment for edge devices

## Activation
1. Fill real nodes and auth settings.
2. Bring up Tailscale on each node.
3. Use `scripts/edge_link_bootstrap.sh` for baseline checks.
