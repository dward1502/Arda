---
sigil: SCROLL
soterion:
  id: bluefin-lmstudio-best-practices
  version: 1.0.0
  classification: general-document
  author: Aulendil
  created: 2026-03-20
  last_edited: 2026-05-03
  status: active
  domain: general
  tags:
    - documentation
    - general
  mnemosyne:
    lineage: bluefin-lmstudio-best-practices-doc
    memory_type: general-knowledge
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# sigil: SCROLL
# Bluefin-LTS & LM Studio Best Practices

## 1. Environment Philosophy
Bluefin-LTS is an immutable OS built on Project Bluefin (Fedora Silverblue/Kinoite). It requires a container-first approach. Modifying the base system (`rpm-ostree`) should be avoided for AI applications.

## 2. LM Studio Installation & Deployment
1. **Use AppImage or Flatpak**: LM Studio provides an AppImage. Download it to `~/Applications/` and make it executable. This runs entirely in user-space without touching the immutable root.
2. **Server Port Configuration**: LM Studio runs its local server by default on port `1234`. 
   - Start the server via the UI or CLI.
   - The API is served at `http://127.0.0.1:1234/v1`.
3. **Hardware Acceleration**: Bluefin ships with out-of-the-box NVIDIA/AMD drivers. Ensure LM Studio detects your GPU (Vulkan or ROCm/CUDA). 

## 3. Llama.cpp CLI (Alternative/Backend)
If running raw `llama.cpp` instead of LM Studio:
- Do not install via `dnf`.
- Compile it inside a `distrobox` container or `toolbx` instance.
  ```bash
  distrobox create --name ai-toolkit --image ghcr.io/ublue-os/ubuntu-toolbox:latest
  distrobox enter ai-toolkit
  sudo apt update && sudo apt install build-essential cmake
  git clone https://github.com/ggerganov/llama.cpp
  cd llama.cpp && make -j
  ```

## 4. Annunimas Integration
- Add the local server to `config/charon.providers.toml` under `base_url = "http://127.0.0.1:1234/v1"`.
- Use Tailscale MagicDNS to share this local server with the rest of the Annunimas fleet.
  - E.g., `http://bluefin-ai.tailnet-name.ts.net:1234/v1`

## 5. Security & Isolation
- The `annunimas-core` processes should connect to `127.0.0.1:1234` locally.
- Do not expose port 1234 to `0.0.0.0` unless Tailscale ACLs are strictly configured to only allow Wardens/Workers.


## See Also
- [annunimas-bluefin-integration.md](annunimas-bluefin-integration.md) - Related documentation
