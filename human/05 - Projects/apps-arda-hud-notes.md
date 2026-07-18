---
title: "Apps Arda Hud Notes"
last_updated: 2026-05-14
soterion:
  type: project_summary
  category: summaries
  project: annunimas
  agent_access: public
  mnemosyne_priority: high
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# ARDA HUD - Audit Notes
**Session:** 2026-03-21
**Author:** Hermes Agent
**Status:** Initial Documentation

---

## OVERVIEW

**Location:** `/apps/arda-hud/`
**Type:** Frontend/Visualization Application
**Tech Stack:** React, Three.js, CSS tools, Tauri (Rust backend)
**Scale:** ~150+ node_modules
**Status:** Actively developed with enhancement plan

**Core Vision:**
> "This is no longer an app. This is your personal Night City where you are the netrunner god-king and every agent is your crew."

**Project Name:** MYTHOS

---

## KEY DOCUMENTATION

### 1. `MYTHOS_SPEC.md` - Master Specification (137 lines)
**Status:** MASTER SPEC - All features to be implemented

**Architecture Layers:**

#### 1.1 Rust Tauri Backend
- Persistent WebSocket server
- Secure auth (API keys + Tailscale/Cloudflare fallback)
- Agent registry: unique cube ID per agent spawn
- Metrics pipeline: gRPC or WS → live updates
  - Health, tokens/sec, decision latency, errors, queue depth

#### 1.2 Multi-Agent Orchestrator Layer
- Hooks: CrewAI / LangGraph / AutoGen / custom Rust agents
- Agent properties in 3D space:
  - Position in 3D space
  - Current task
  - Last 10 actions
  - Connections to other agents
- Cloud extension: Modal / Fly.io / AWS agents → same WS endpoint

#### 1.3 State & Sync Engine
- Zustand + WebSocket sync (local-first)
- Sync on reconnect
- Offline mode: Tauri SQLite cache
- Remote access: <300ms to home PC (or cloud relay)

#### 1.4 Voice Control
- Web Speech API → local Ollama/LLM parser
- Examples: "SUPER, analyze Q3 metrics and spawn a research agent"
- Franky voice response: "SUUUUUPER — agent deployed, choom!"

#### 1.5 3D File Trees = Living Nodes
- Folders = tall glowing obelisks/towers (height = file count)
- Files = smaller cubes orbiting/stacked inside
- Click folder → smooth expansion animation
- Drag & drop = fs::rename via Tauri
- Agents fly between towers with glowing trails

#### 1.6 Live Agent Action Visualization
- Particle paths + floating action bubbles
- Example: "read contract.pdf → summarized → passed to legal-agent"
- Hover any cube → instant replay timeline hologram

#### 1.7 Document Editing
- Double-click file cube → holographic floating pane
- Monaco Editor + live preview pane
- Code flows as scrolling data ribbons
- AI highlights glow magenta on code

#### 1.8 Casual Conversation Zone
- "Chill Spire" in back of metropolis
- Low neon lounge, Franky-Rache avatar
- Voice or text, zero task pressure

#### 1.9 Instant Section Switching
- Keyboard 1-9 or voice command or floating neon signs:
  - 1 = Core Metropolis (agents)
  - 2 = File Forest
  - 3 = Business District (metrics/economy)
  - 4 = Security Black Ice
  - 5 = Lounge (casual)
- Camera tweens with cinematic bloom flash

#### 1.10 Remote-First Design
- Coffee shop laptop → "Connect to Home Base" → secure WS tunnel
- Home PC runs heavy agents + file system
- Cloud agents auto-appear as satellite towers with data beams
- Works on any machine, any network

#### 1.11 Extra Ideas (God-Tier)
- Git as neon rivers — branches = glowing flowing streams
- Decision trace graphs — 3D mind-map of every agent thought
- In-scene floating terminal — cyberpunk font
- Agent marketplace stall — drag-and-drop templates
- Live shader dashboards — metrics on cube surfaces (custom GLSL)
- Multi-window Tauri — one pure 3D, one inspector
- AR toggle — WebXR mode (glasses later)

---

### 2. `ENHANCEMENT_PLAN.md` - Feature Roadmap (129 lines)

#### Phase 1: Data Ingestion System

**1.1 Unified Data Parser**
- Input Types:
  - JSON (API responses, state files)
  - Markdown (docs, specs, GDDs)
  - YAML (configs, policies)
  - CSV/TSV (tabular data)
  - Graph formats (DOT, JSON-graph)
- Output: Normalized component payload

**1.2 Component Registry**
| Component | Data Input | Visual Style |
|-----------|------------|--------------|
| KanbanCard | { title, description, status, tags } | Draggable card with lanes |
| DataGrid | Array of objects | Sortable table with filters |
| MetricPill | { value, label, trend } | Glowing stat chip |
| Timeline | { events[], timestamp } | Vertical/Horizontal timeline |
| Graph3D | { nodes[], edges[] } | Three.js force graph |
| DocumentReader | Markdown/HTML | Styled content viewer |
| StatusMatrix | { items[], states[] } | Grid with state colors |
| StreamLog | { lines[] } | Terminal-style feed |

**1.3 Ingestion API**
```typescript
interface IngestRequest {
  source: string | URL | File
  format: 'auto' | 'json' | 'markdown' | 'yaml' | 'csv'
  componentType: ComponentType
  options?: RenderOptions
}
```

#### Phase 2: Dynamic Component Renderer

**2.1 Slot System**
- Views contain named slots: `main`, `sidebar`, `overlay`, `floating`
- Components can be slotted into any view
- Slot layout persisted per-user

**2.2 Data Binding**
- Components auto-bind to data sources
- Live updates via WebSocket/polling
- Manual refresh option

#### Phase 3: Edit Mode

**3.1 Activation**
- Toggle: `E` key or UI button
- Visual indicator: subtle glow on editable elements

**3.2 Editable Actions**
| Action | Mouse | Gesture |
|--------|-------|---------|
| Move component | Drag | Grab + move |
| Resize | Drag handles | Pinch/stretch |
| Duplicate | Ctrl+D | Two-finger tap |
| Delete | X button | Swipe away |
| Change slot | Drag to slot | Drop on slot |

**3.3 Layout Engine**
- CSS Grid/Flexbox with drag library (dnd-kit or react-dnd)
- Snap-to-grid option
- Min/max size constraints per component
- Collision detection

#### Phase 4: Gesture-Ready Architecture

**4.1 Hitbox System**
```typescript
interface GestureHitbox {
  id: string
  bounds: Rect2D
  actions: GestureAction[]
  feedback: 'glow' | 'scale' | 'color'
}
```

**4.2 VR/AR Integration Points**
- Ray-casting targets match hitboxes
- Hand tracking: grab, point, pinch mapped to actions
- Eye tracking for focus

**4.3 Spatial Layout**
- 3D positioning option (behind/in-front)
- Depth-based scaling
- Gaze activation zones

#### Implementation Priority
1. **Data Parser** - Core ingestion
2. **Component Registry** - Basic set (Card, Grid, Metric)
3. **Edit Mode** - Drag/drop foundation
4. **Slot System** - View integration
5. **Gesture Hitboxes** - VR-ready structure
6. **Additional Components** - As needed

---

### 3. `HUD_EVENT_SCHEMA.md` - Event Schema (105 lines)

**Status:** Active v1.0.0
**Purpose:** Standardizes all log/combat-feed events in ARDA HUD

#### Core Fields
```ts
{
  id: string
  timestamp: Date
  severity: 'debug' | 'info' | 'warn' | 'error' | 'critical'
  source: string
  message: string
  refs: Array<{ type: 'agent' | 'mission' | 'node'; id: string }>
  pinned: boolean
  schemaVersion: '1.0.0'
  kind: string
  domain: 'system' | 'operations' | 'agent' | 'mission' | 'inventory' | 'dock' | 'governance' | 'security' | 'telemetry' | 'knowledge' | 'external' | 'unknown'
}
```

#### Optional Extensions
```ts
{
  sigils?: string[]              // Soterion/UI signal tags, e.g., ['∇','⚡','◈']
  tags?: string[]                // Fast filters/grouping
  metrics?: {
    jwEstimated?: number         // JouleWork estimate
    leScore?: number             // Love Equation score
    triadScore?: number          // Triad aggregate score
    durationMs?: number          // Execution latency
  }
  trace?: {
    taskId?: string
    missionId?: string
    agentId?: string
    correlationId?: string
  }
  raw?: Record<string, unknown>  // Original event payload for audit/debug
}
```

#### Event Kinds (initial)
- `operations.system_action` - action-bus execution results
- `telemetry.<operation>` - parsed JSONL telemetry streams
- `telemetry.joulework` - legacy JouleWork log parsing fallback
- `<domain>.event` - default when producer didn't set kind

#### Ingestion Rules
- All producers pass through `normalizeHudEvent(...)`
- Missing fields defaulted (id, timestamp, schemaVersion, domain, kind)
- `setEvents(...)` and `addEvent(...)` normalize every event
- `arda:system-action-result` browser events → schema events
- Native telemetry merge reads from:
  - `data/hades/hades_log.jsonl`
  - `data/hades/joulework.jsonl`
  - `data/athena/digest.jsonl`
  - `data/athena/deep_graph.jsonl`
  - `data/prometheus/orders.jsonl`
  - `data/prometheus/escalations.jsonl`
  - `data/prometheus/autopilot/metrics.jsonl`

#### Feed Health Snapshot
HUD computes per-feed observability metrics each refresh cycle:
```ts
{
  feedId: string
  status: 'healthy' | 'stale' | 'offline' | 'error'
  lastEventAt: Date | null
  lastEventAgeSec: number | null
  linesRead: number
  eventsParsed: number
  parseErrors: number
}
```

#### Example Event
```json
{
  "id": "action-status_report-1741511200000",
  "timestamp": "2026-03-09T20:13:20.000Z",
  "severity": "info",
  "source": "action:voice",
  "message": "Action status_report completed via weathertop-http",
  "refs": [],
  "pinned": false,
  "schemaVersion": "1.0.0",
  "kind": "operations.system_action",
  "domain": "operations",
  "sigils": ["∇", "⚡"],
  "tags": ["status_report", "frankyrache", "success", "weathertop-http"],
  "metrics": { "durationMs": 215 },
  "trace": { "correlationId": "status_report:1741511200000" }
}
```

---

## ADDITIONAL FILES

### `README.md` - Project overview
- Entry point for developers
- Quickstart guide

### `RUNTIME.md` - Runtime specifications
- Performance targets
- Resource requirements

### `SPEC.md` - Additional specifications
- Component specs
- Integration specs

### `SYSTEM_SPECS.md` - System specifications
- Hardware requirements
- Software dependencies

### Scene Documentation
- `src/scene/README.md` - Scene overview
- `src/scene/world/README.md` - World scene
- `src/scene/workstations/README.md` - Workstation scenes
- `src/scene/boardroom/README.md` - Boardroom scene
- `src/scene/systems/README.md` - Systems scene
- `src/scene/shaders/README.md` - Shaders documentation
- `src/assets/scene/README.md` - Scene assets

---

## KEY FEATURES SUMMARY

1. **3D Visualization** - Three.js-based metropolis
2. **Multi-Agent Support** - CrewAI/LangGraph/AutoGen integration
3. **Voice Control** - Speech API with LLM parsing
4. **File System Integration** - 3D file trees with drag-and-drop
5. **Live Agent Tracking** - Particle paths, action bubbles
6. **Document Editing** - Monaco Editor in 3D space
7. **Remote Access** - WebSocket tunnels, offline mode
8. **Event System** - Unified schema with 12 domains
9. **Component System** - 8+ component types with enhancement plan
10. **VR/AR Ready** - Gesture hitbox architecture

---

## VISUAL AESTHETICS

- **Style:** Cyberpunk/Neon
- **References:** stockcake.com, fanruan.com, designbundles.net
- **Elements:**
  - Glowing obelisks/towers
  - Floating data ribbons
  - Particle trails
  - Cinematic bloom effects
  - Neon district signs

---

## NOTABLE QUOTES

> "This is your personal Night City where you are the netrunner god-king."

> "Every agent is your crew"

> "SUUUUUPER — agent deployed, choom!"

---

## NEXT STEPS

- Dive into scene documentation
- Review enhancement plan implementation status
- Check runtime specs
- Examine shader documentation
- Audit scene assets

---

**End of ARDA HUD Notes**
