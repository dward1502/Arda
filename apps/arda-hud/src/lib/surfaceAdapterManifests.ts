// sigil: REPAIR
import type { ArdaWorkstationManifest } from './ardaSource'

export type SurfaceAdapterKind = 'arda' | 'third_party_service' | 'local_service' | 'media_adapter' | 'session_adapter' | 'custom' | 'research_book'
export type SurfaceAdapterTrust = 'operator_curated' | 'athena_researched' | 'unverified'
export type SurfaceAdapterFocusMode = 'native_window' | 'external_browser' | 'inline_embed' | 'preview_only'

export interface SurfaceAdapterManifest {
  id: string
  title: string
  provider: string
  kind: SurfaceAdapterKind
  sourceZoneId: string
  summary: string
  externalUrl?: string
  embedUrl?: string
  allowInlineEmbed: boolean
  preferredFocusMode: SurfaceAdapterFocusMode
  previewRows: Array<{ label: string; value: string }>
  capabilities: string[]
  readiness: Array<{ label: string; status: 'ready' | 'planned' | 'blocked'; detail: string }>
  freshnessSource: 'manual' | 'athena_library_book' | 'live_api' | 'unknown'
  trust: SurfaceAdapterTrust
  tags: string[]
}

export interface SurfaceAdapterFocusContract {
  sourceZoneId: string
  focusMode: SurfaceAdapterFocusMode
  target: string | null
  inlineStatus: 'allowed' | 'blocked' | 'unavailable'
  reason: string
}

export const SURFACE_ADAPTER_MANIFESTS: SurfaceAdapterManifest[] = [
  {
    id: 'factory_ai_service_surface',
    title: 'Factory AI Surface',
    provider: 'Factory',
    kind: 'third_party_service',
    sourceZoneId: 'service_factory_ai',
    summary: 'External agentic software-development platform reference surface for missions, droids, and operator workflow comparison.',
    externalUrl: 'https://factory.ai/',
    allowInlineEmbed: false,
    preferredFocusMode: 'external_browser',
    previewRows: [
      { label: 'surface', value: 'missions' },
      { label: 'entry', value: 'external' },
      { label: 'adapter', value: 'manual' },
    ],
    capabilities: ['Mission planning reference', 'Agent workflow comparison', 'Future embed or API adapter candidate'],
    readiness: [
      { label: 'Open service', status: 'ready', detail: 'Launches the public service URL in an external browser.' },
      { label: 'Inline embed', status: 'blocked', detail: 'Blocked until framing/auth policy is explicitly approved.' },
      { label: 'Live session bridge', status: 'planned', detail: 'Requires an adapter manifest backed by ATHENA/Mnemosyne research.' },
    ],
    freshnessSource: 'manual',
    trust: 'operator_curated',
    tags: ['agentic-development', 'missions', 'third-party'],
  },
  {
    id: 'warp_dev_service_surface',
    title: 'Warp Surface',
    provider: 'Warp',
    kind: 'third_party_service',
    sourceZoneId: 'service_warp_dev',
    summary: 'External terminal and agent-control-plane reference surface for local/cloud session continuity and operator join/steer workflows.',
    externalUrl: 'https://www.warp.dev/',
    allowInlineEmbed: false,
    preferredFocusMode: 'external_browser',
    previewRows: [
      { label: 'surface', value: 'terminal' },
      { label: 'entry', value: 'external' },
      { label: 'adapter', value: 'manual' },
    ],
    capabilities: ['Agent terminal reference', 'Local/cloud session model', 'Future attach-session cockpit surface'],
    readiness: [
      { label: 'Open Warp', status: 'ready', detail: 'Launches Warp in an external browser.' },
      { label: 'Inline embed', status: 'blocked', detail: 'Blocked until Warp framing/auth policy is approved.' },
      { label: 'Attach session', status: 'planned', detail: 'Future adapter can bind a live session or local command handoff.' },
    ],
    freshnessSource: 'manual',
    trust: 'operator_curated',
    tags: ['terminal', 'agent-sessions', 'third-party'],
  },
  {
    id: 'vast_ai_os_service_surface',
    title: 'VAST AI OS Surface',
    provider: 'VAST Data',
    kind: 'third_party_service',
    sourceZoneId: 'service_vast_ai_os',
    summary: 'External AI data-plane reference surface for storage, database, compute, and infrastructure operating-model comparison.',
    externalUrl: 'https://www.vastdata.com/platform/ai-os',
    allowInlineEmbed: false,
    preferredFocusMode: 'external_browser',
    previewRows: [
      { label: 'surface', value: 'data plane' },
      { label: 'entry', value: 'external' },
      { label: 'adapter', value: 'manual' },
    ],
    capabilities: ['AI data-plane reference', 'Infrastructure operating-model comparison', 'Future storage/runtime adapter candidate'],
    readiness: [
      { label: 'Open VAST', status: 'ready', detail: 'Launches the public VAST AI OS page externally.' },
      { label: 'Inline embed', status: 'blocked', detail: 'Blocked until framing/auth policy is explicitly approved.' },
      { label: 'Live telemetry', status: 'planned', detail: 'Requires a real data-plane integration or imported projection.' },
    ],
    freshnessSource: 'manual',
    trust: 'operator_curated',
    tags: ['data-plane', 'infrastructure', 'third-party'],
  },
  {
    id: 'beelink_grafana_service_surface',
    title: 'Beelink Grafana',
    provider: 'Grafana',
    kind: 'local_service',
    sourceZoneId: 'service_beelink_grafana',
    summary: 'Local fleet observability surface hosted on Beelink for dashboards, Manwe routing, cluster health, and autonomy queue telemetry.',
    externalUrl: 'http://100.103.125.88:3000',
    embedUrl: 'http://100.103.125.88:3000',
    allowInlineEmbed: false,
    preferredFocusMode: 'native_window',
    previewRows: [
      { label: 'host', value: 'beelink' },
      { label: 'port', value: '3000' },
      { label: 'inline', value: 'disabled' },
    ],
    capabilities: ['Fleet observability', 'Prometheus dashboards', 'Routing and cluster health review'],
    readiness: [
      { label: 'Open Grafana', status: 'ready', detail: 'Opens the Beelink Grafana URL in a focused/native surface.' },
      { label: 'Inline embed', status: 'blocked', detail: 'Keep disabled until Grafana frame/CSP behavior is proven in native Tauri WebKit.' },
      { label: 'Dashboard presets', status: 'planned', detail: 'Bind specific dashboard URLs after service presets are added to Settings.' },
    ],
    freshnessSource: 'manual',
    trust: 'operator_curated',
    tags: ['local-service', 'beelink', 'grafana', 'observability'],
  },
  {
    id: 'beelink_open_webui_service_surface',
    title: 'Beelink Open WebUI',
    provider: 'Open WebUI',
    kind: 'local_service',
    sourceZoneId: 'service_beelink_openwebui',
    summary: 'Local model/chat interface candidate hosted with the Beelink service stack for operator LLM sessions and future agent activity display.',
    externalUrl: 'http://100.103.125.88:8080',
    embedUrl: 'http://100.103.125.88:8080',
    allowInlineEmbed: false,
    preferredFocusMode: 'native_window',
    previewRows: [
      { label: 'host', value: 'beelink' },
      { label: 'port', value: '8080' },
      { label: 'inline', value: 'disabled' },
    ],
    capabilities: ['Local model UI', 'Operator chat surface', 'Future agent-session review'],
    readiness: [
      { label: 'Open WebUI', status: 'ready', detail: 'Opens the configured Beelink Open WebUI candidate URL.' },
      { label: 'Inline embed', status: 'blocked', detail: 'Keep disabled until auth, frame policy, and WebKit behavior are verified.' },
      { label: 'Live agent view', status: 'planned', detail: 'Requires a separate event/session adapter before showing live agent work.' },
    ],
    freshnessSource: 'manual',
    trust: 'operator_curated',
    tags: ['local-service', 'beelink', 'open-webui', 'models'],
  },
  {
    id: 'arda_media_library_surface',
    title: 'ARDA Media Library',
    provider: 'ARDA',
    kind: 'media_adapter',
    sourceZoneId: 'media_library',
    summary: 'Focused document and media review surface for markdown, PDFs, images, video references, and operator-curated packets.',
    allowInlineEmbed: false,
    preferredFocusMode: 'native_window',
    previewRows: [
      { label: 'documents', value: 'md / pdf / doc' },
      { label: 'media', value: 'image / video' },
      { label: 'focus', value: 'native window' },
    ],
    capabilities: ['Markdown packet preview', 'PDF/image/video reference slot', 'Future scoped media opener'],
    readiness: [
      { label: 'Compact previews', status: 'ready', detail: 'Boardroom screens can declare document and media preview widgets.' },
      { label: 'Focused review', status: 'planned', detail: 'Native focused viewer needs scoped file resolution and codec policy before playback/rendering.' },
      { label: 'Inline media', status: 'blocked', detail: 'Inline rendering remains disabled until source, codec, and redaction policy are explicit.' },
    ],
    freshnessSource: 'manual',
    trust: 'operator_curated',
    tags: ['media', 'documents', 'local-files', 'focused-adapter'],
  },
  {
    id: 'arda_remote_session_surface',
    title: 'Agent Remote Session',
    provider: 'ARDA',
    kind: 'session_adapter',
    sourceZoneId: 'agent_remote_session',
    summary: 'Focused remote observation surface for agent activity in tools such as Blender, ComfyUI, browser sessions, or VM desktops.',
    allowInlineEmbed: false,
    preferredFocusMode: 'native_window',
    previewRows: [
      { label: 'session', value: 'agent desktop' },
      { label: 'transport', value: 'noVNC/WebRTC pending' },
      { label: 'focus', value: 'native window' },
    ],
    capabilities: ['Agent session placeholder', 'Remote desktop readiness gate', 'Future noVNC/WebRTC bridge target'],
    readiness: [
      { label: 'Compact previews', status: 'ready', detail: 'Boardroom screens can declare remote-session preview widgets.' },
      { label: 'Transport bridge', status: 'planned', detail: 'Requires explicit noVNC/WebRTC endpoint, auth, and lifecycle contract.' },
      { label: 'Inline remote view', status: 'blocked', detail: 'Remote sessions must stay out of inline previews until transport and isolation are verified.' },
    ],
    freshnessSource: 'manual',
    trust: 'operator_curated',
    tags: ['remote-session', 'agent-activity', 'vm', 'focused-adapter'],
  },
]

export const SURFACE_ADAPTER_WORKSTATION_MANIFESTS: ArdaWorkstationManifest[] = SURFACE_ADAPTER_MANIFESTS.map((manifest) => ({
  id: `${manifest.sourceZoneId}_workstation`,
  title: manifest.title,
  source_zone_id: manifest.sourceZoneId,
  entry_anchor_id: `${manifest.sourceZoneId}_entry`,
  module_ids: manifest.sourceZoneId === 'media_library' ? ['media_library', 'service_embed'] : ['service_embed'],
  presentation_modes: ['in_scene', 'native_window'],
}))

export function getSurfaceAdapterManifest(sourceZoneId: string | null | undefined): SurfaceAdapterManifest | null {
  if (!sourceZoneId) return null
  return SURFACE_ADAPTER_MANIFESTS.find((manifest) => manifest.sourceZoneId === sourceZoneId) ?? null
}

export function getSurfaceAdapterFocusContract(sourceZoneId: string | null | undefined): SurfaceAdapterFocusContract | null {
  const manifest = getSurfaceAdapterManifest(sourceZoneId)
  if (!manifest) return null
  const target = manifest.embedUrl ?? manifest.externalUrl ?? null
  const inlineStatus = manifest.allowInlineEmbed && manifest.embedUrl
    ? 'allowed'
    : manifest.embedUrl || manifest.externalUrl
      ? 'blocked'
      : 'unavailable'
  return {
    sourceZoneId: manifest.sourceZoneId,
    focusMode: manifest.preferredFocusMode,
    target,
    inlineStatus,
    reason: manifest.allowInlineEmbed
      ? 'Inline embedding is explicitly allowed for this adapter.'
      : manifest.preferredFocusMode === 'preview_only'
        ? 'This adapter is a preview-only declaration until a focused target exists.'
        : 'Use the focused surface path; inline embedding is disabled until policy and runtime behavior are verified.',
  }
}

export function getSurfaceAdapterWorkstationManifests(): ArdaWorkstationManifest[] {
  return [...SURFACE_ADAPTER_WORKSTATION_MANIFESTS]
}
