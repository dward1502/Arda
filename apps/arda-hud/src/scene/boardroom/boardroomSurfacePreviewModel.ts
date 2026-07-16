import type {
  BoardroomSurfaceAdapterType,
  BoardroomSurfaceLayout,
  BoardroomSurfaceWidget,
  BoardroomSurfaceWidgetKind,
} from '../../lib/boardroomSlotSettings'
import type { ArdaFreshnessState, ArdaSourceProvenance } from '../../lib/ardaProvenance'
import { getRefreshAffordance, type ArdaRefreshAffordanceSafety } from '../../lib/ardaProvenance'

export type BoardroomSurfacePreviewTone = 'cyan' | 'violet' | 'gold' | 'mint' | 'rose'
export type BoardroomSurfacePreviewStatus = 'nominal' | 'external' | 'attention' | 'disabled'

export interface BoardroomSurfacePreviewProvenanceModel {
  state: ArdaFreshnessState
  label: string
  sourcePath: string
  sourceKind: ArdaSourceProvenance['sourceKind']
  refreshSafety: ArdaRefreshAffordanceSafety
}

export interface BoardroomSurfacePreviewWidgetModel {
  id: string
  kind: BoardroomSurfaceWidgetKind
  title: string
  binding: string
  gridArea: string
  mediaLabel: string
  signal: number
  status: BoardroomSurfacePreviewStatus
  values: number[]
}

export interface BoardroomSurfacePreviewModel {
  title: string
  eyebrow: string
  tone: BoardroomSurfacePreviewTone
  status: BoardroomSurfacePreviewStatus
  glyph: string
  refreshMs: number
  provenance?: BoardroomSurfacePreviewProvenanceModel
  widgets: BoardroomSurfacePreviewWidgetModel[]
}

function hashString(value: string): number {
  let hash = 0
  for (let index = 0; index < value.length; index += 1) {
    hash = (hash * 31 + value.charCodeAt(index)) % 9973
  }
  return hash
}

function valuesForWidget(widget: BoardroomSurfaceWidget): number[] {
  const seed = hashString(`${widget.id}:${widget.kind}:${widget.data_binding}`)
  return [0, 1, 2, 3].map((offset) => ((seed + offset * 173) % 100) / 100)
}

function mediaLabelForWidget(widget: BoardroomSurfaceWidget): string {
  if (widget.kind === 'markdown_doc') return 'MD'
  if (widget.kind === 'pdf_doc') return 'PDF'
  if (widget.kind === 'image_asset') return 'IMG'
  if (widget.kind === 'video_asset') return 'MOV'
  if (widget.kind === 'document_asset') return 'DOC'
  if (widget.kind === 'data_stream') return 'DATA'
  if (widget.kind === 'remote_session') return 'VM'
  if (widget.kind === 'agent_comms') return 'COMMS'
  if (widget.kind === 'iframe_preview') return 'WEB'
  if (widget.kind === 'media_tile') return 'MEDIA'
  return widget.kind.split('_')[0]?.toUpperCase() ?? 'WGT'
}

function toneForAdapter(adapterType: BoardroomSurfaceAdapterType): BoardroomSurfacePreviewTone {
  if (adapterType === 'service_embed' || adapterType === 'external_url') return 'violet'
  if (adapterType === 'agent_activity' || adapterType === 'streaming_text') return 'mint'
  if (adapterType === 'remote_desktop' || adapterType === 'media_viewer') return 'gold'
  return 'cyan'
}

function glyphForAdapter(adapterType: BoardroomSurfaceAdapterType): string {
  if (adapterType === 'service_embed') return 'SVC'
  if (adapterType === 'external_url') return 'URL'
  if (adapterType === 'agent_activity') return 'AGT'
  if (adapterType === 'streaming_text') return 'TXT'
  if (adapterType === 'remote_desktop') return 'VM'
  if (adapterType === 'media_viewer') return 'MED'
  return 'GRID'
}

function statusForWidget(layout: BoardroomSurfaceLayout, widget: BoardroomSurfaceWidget): BoardroomSurfacePreviewStatus {
  if (!layout.enabled) return 'disabled'
  if (widget.kind === 'iframe_preview' && !layout.embed.allow_inline) return 'attention'
  if (widget.kind === 'remote_session' && layout.focus.mode !== 'native_window') return 'attention'
  if (layout.adapter_type === 'service_embed' || layout.adapter_type === 'external_url') return 'external'
  return 'nominal'
}

function normalizeMatcher(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '')
}

function provenancePriority(state: ArdaFreshnessState): number {
  switch (state) {
    case 'missing':
      return 0
    case 'blocked':
      return 1
    case 'stale':
      return 2
    case 'unknown':
      return 3
    case 'derived':
      return 4
    case 'fresh':
      return 5
    default:
      return 6
  }
}

function provenanceForLayout(
  layout: BoardroomSurfaceLayout,
  sourceProvenance: ArdaSourceProvenance[] | undefined,
): BoardroomSurfacePreviewProvenanceModel | undefined {
  if (!sourceProvenance?.length) return undefined

  const bindings = [
    layout.focus.target,
    layout.adapter_type,
    ...layout.preview.widgets.flatMap((widget) => [widget.id, widget.data_binding, widget.title]),
  ].map(normalizeMatcher).filter(Boolean)

  const matches = sourceProvenance.filter((record) => {
    const recordText = normalizeMatcher([record.domainId, record.label, ...record.sourcePaths].join(' '))
    return bindings.some((binding) => recordText.includes(binding) || binding.includes(recordText))
  })

  const record = matches.sort((left, right) => {
    const priority = provenancePriority(left.state) - provenancePriority(right.state)
    if (priority !== 0) return priority
    return left.label.localeCompare(right.label)
  })[0]

  if (!record) return undefined
  return {
    state: record.state,
    label: record.label,
    sourcePath: record.sourcePaths[0] ?? 'source path unavailable',
    sourceKind: record.sourceKind,
    refreshSafety: getRefreshAffordance(record).safety,
  }
}

const ROLE_WIDGET_FAMILIES = new Set(['fleet', 'systems_health', 'routing_health', 'sovereign_world'])

function roleFallbackWidgetsForRole(role: string | undefined): BoardroomSurfaceWidget[] {
  const family = ROLE_WIDGET_FAMILIES.has(role ?? '') ? 'fleet' : 'generic'
  if (family !== 'fleet') return []

  return [
    { id: 'role.fleet.status_grid', kind: 'status_grid', title: 'Live fleet status', data_binding: `${role ?? 'fleet'}.live_status`, grid_area: 'main' },
    { id: 'role.fleet.sparkline', kind: 'sparkline', title: 'Route latency', data_binding: `${role ?? 'fleet'}.route_latency`, grid_area: 'main' },
    { id: 'role.fleet.providers', kind: 'particle_stream', title: 'Provider bars', data_binding: `${role ?? 'fleet'}.providers`, grid_area: 'main' },
    { id: 'role.fleet.freshness', kind: 'agent_comms', title: 'Source freshness', data_binding: `${role ?? 'fleet'}.source_freshness`, grid_area: 'main' },
  ]
}

export function deriveBoardroomSurfacePreviewModel({
  title,
  layout,
  sourceProvenance,
  sourceZoneId,
}: {
  title: string
  layout: BoardroomSurfaceLayout
  sourceProvenance?: ArdaSourceProvenance[]
  sourceZoneId?: string
}): BoardroomSurfacePreviewModel {
  const roleFallbackWidgets = roleFallbackWidgetsForRole(sourceZoneId)
  const widgetSource = roleFallbackWidgets.length > 0 ? roleFallbackWidgets : layout.preview.widgets
  const widgets = widgetSource.map((widget) => ({
    id: widget.id,
    kind: widget.kind,
    title: widget.title,
    binding: widget.data_binding,
    gridArea: widget.grid_area,
    mediaLabel: mediaLabelForWidget(widget),
    signal: valuesForWidget(widget)[0],
    status: statusForWidget(layout, widget),
    values: valuesForWidget(widget),
  }))
  const provenance = provenanceForLayout(layout, sourceProvenance)
  const provenanceNeedsAttention = provenance
    ? ['stale', 'missing', 'blocked', 'unknown'].includes(provenance.state)
    : false
  const status: BoardroomSurfacePreviewStatus = !layout.enabled
    ? 'disabled'
    : widgets.some((widget) => widget.status === 'attention') || provenanceNeedsAttention
      ? 'attention'
      : layout.adapter_type === 'service_embed' || layout.adapter_type === 'external_url'
        ? 'external'
        : roleFallbackWidgets.length > 0
          ? !layout.enabled
            ? 'disabled'
            : 'nominal'
          : 'nominal'

  return {
    title,
    eyebrow: layout.preview.mode.replace(/_/g, ' ').toUpperCase(),
    tone: !layout.enabled ? 'rose' : toneForAdapter(layout.adapter_type),
    status,
    glyph: glyphForAdapter(layout.adapter_type),
    refreshMs: layout.preview.refresh_ms,
    provenance,
    widgets,
  }
}
