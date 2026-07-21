// sigil: REPAIR
import { useMemo, useState } from 'react'
import { Gauge, Route, ShieldCheck, SlidersHorizontal, WalletCards } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import type { MonitorAssignment, ThemeId, ViewMode } from '../core/types'
import type { JsonRecord } from '../../../lib/ardaSource'
import type {
  BoardroomSurfaceAdapterType,
  BoardroomSurfaceFocusMode,
  BoardroomSurfaceLayout,
  BoardroomSurfaceWidget,
  BoardroomSurfaceWidgetKind,
  BoardroomRoleAssignmentProfile,
} from '../../../lib/boardroomSlotSettings'
import {
  applyConfigProfile,
  getConfigProfiles,
  previewConfigProfile,
  type ConfigProjectionPreview,
} from '../../../lib/configWalkthrough'

interface ConfigProfile {
  id: string
  title: string
  glyph: string
  productPosture: string
  summary: string
  settings: Record<string, string | number>
  writes: string[]
  nextAction: string
}

interface ConfigDimension {
  id: string
  label: string
  kind: string
  canonicalHome: string
}

interface SettingsModuleProps {
  theme: ThemeId
  editMode: boolean
  viewMode: ViewMode
  themeOptions: Array<{ id: ThemeId; label: string }>
  monitorAssignments: MonitorAssignment[]
  roleAssignmentProfiles?: BoardroomRoleAssignmentProfile[]
  worldSurfaceAssignments?: MonitorAssignment[]
  futureDomains: Array<{ title: string; status: string }>
  configWalkthrough: JsonRecord | null
  rootPath: string | null
  onConfigApplied?: () => void
  onUpdateBoardroomAssignment?: (slotId: string, sourceZoneId: string) => void
  onUpdateSurfaceLayout?: (slotId: string, updater: BoardroomSurfaceLayout | ((current: BoardroomSurfaceLayout) => BoardroomSurfaceLayout)) => void
  onUpdateWorldSurfaceLayout?: (surfaceId: string, updater: BoardroomSurfaceLayout | ((current: BoardroomSurfaceLayout) => BoardroomSurfaceLayout)) => void
  onToggleEditMode: () => void
}

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as JsonRecord
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function getString(value: unknown, fallback = 'n/a'): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback
}

function getNumber(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function toDimension(value: unknown): ConfigDimension | null {
  const record = asRecord(value)
  if (!record) return null
  return {
    id: getString(record.id, ''),
    label: getString(record.label, 'Config'),
    kind: getString(record.kind, 'field'),
    canonicalHome: getString(record.canonical_home, 'unknown'),
  }
}

function formatSettingLabel(value: string): string {
  return value
    .split('_')
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ')
}

const DIMENSION_ICONS = {
  runtime_mode: Route,
  autonomy_level: Gauge,
  node_visibility: SlidersHorizontal,
  provider_budget: WalletCards,
  governance_strictness: ShieldCheck,
} as const

const SURFACE_ADAPTER_TYPES: BoardroomSurfaceAdapterType[] = [
  'component_grid',
  'external_url',
  'service_embed',
  'media_viewer',
  'streaming_text',
  'remote_desktop',
  'agent_activity',
]

const SURFACE_FOCUS_MODES: BoardroomSurfaceFocusMode[] = [
  'in_scene_workstation',
  'native_window',
  'external_browser',
  'inline_embed',
]

const SURFACE_WIDGET_KINDS: BoardroomSurfaceWidgetKind[] = [
  'metric_strip',
  'particle_stream',
  'sparkline',
  'status_grid',
  'agent_comms',
  'media_tile',
  'iframe_preview',
  'markdown_doc',
  'pdf_doc',
  'image_asset',
  'video_asset',
  'document_asset',
  'data_stream',
  'remote_session',
]

const SURFACE_SERVICE_PRESETS: Array<{
  id: string
  label: string
  sourceZoneId: string
  adapterType: BoardroomSurfaceAdapterType
  previewMode: BoardroomSurfaceLayout['preview']['mode']
  embedUrl: string | null
  widgets: BoardroomSurfaceWidget[]
}> = [
  {
    id: 'beelink_grafana',
    label: 'Beelink Grafana',
    sourceZoneId: 'service_beelink_grafana',
    adapterType: 'service_embed',
    previewMode: 'service_status',
    embedUrl: 'http://100.103.125.88:3000',
    widgets: [
      { id: 'grafana.status', kind: 'status_grid', title: 'Grafana Status', data_binding: 'service_beelink_grafana.health', grid_area: 'top' },
      { id: 'grafana.metrics', kind: 'metric_strip', title: 'Fleet Metrics', data_binding: 'prometheus.fleet.summary', grid_area: 'main' },
      { id: 'grafana.spark', kind: 'sparkline', title: 'Cluster Trend', data_binding: 'grafana.cluster.trend', grid_area: 'side' },
    ],
  },
  {
    id: 'beelink_openwebui',
    label: 'Beelink Open WebUI',
    sourceZoneId: 'service_beelink_openwebui',
    adapterType: 'service_embed',
    previewMode: 'service_status',
    embedUrl: 'http://100.103.125.88:8080',
    widgets: [
      { id: 'openwebui.status', kind: 'status_grid', title: 'WebUI Status', data_binding: 'service_beelink_openwebui.health', grid_area: 'top' },
      { id: 'openwebui.comms', kind: 'agent_comms', title: 'Session Feed', data_binding: 'openwebui.session.feed', grid_area: 'main' },
      { id: 'openwebui.models', kind: 'metric_strip', title: 'Model Lane', data_binding: 'openwebui.models.summary', grid_area: 'side' },
    ],
  },
  {
    id: 'hermes_dashboard',
    label: 'Hermes Dashboard',
    sourceZoneId: 'hermes_dashboard',
    adapterType: 'service_embed',
    previewMode: 'stream_feed',
    embedUrl: 'http://127.0.0.1:9119',
    widgets: [
      { id: 'hermes.terminal', kind: 'agent_comms', title: 'Hermes Terminal', data_binding: 'hermes.dashboard.status', grid_area: 'main' },
      { id: 'hermes.dispatch', kind: 'status_grid', title: 'Dispatch', data_binding: 'hermes.dispatch.health', grid_area: 'side' },
    ],
  },
  {
    id: 'media_library',
    label: 'Media Library',
    sourceZoneId: 'media_library',
    adapterType: 'media_viewer',
    previewMode: 'media_thumbnail',
    embedUrl: null,
    widgets: [
      { id: 'media.markdown', kind: 'markdown_doc', title: 'Markdown', data_binding: 'docs/plans/*.md', grid_area: 'top' },
      { id: 'media.pdf', kind: 'pdf_doc', title: 'PDF Packet', data_binding: 'docs/**/*.pdf', grid_area: 'main' },
      { id: 'media.image', kind: 'image_asset', title: 'Reference Image', data_binding: 'data/media/**/*', grid_area: 'side' },
      { id: 'media.video', kind: 'video_asset', title: 'Video Clip', data_binding: 'data/media/**/*.mov', grid_area: 'side' },
    ],
  },
  {
    id: 'agent_remote_session',
    label: 'Agent Remote Session',
    sourceZoneId: 'agent_remote_session',
    adapterType: 'remote_desktop',
    previewMode: 'remote_preview',
    embedUrl: null,
    widgets: [
      { id: 'session.remote', kind: 'remote_session', title: 'Agent Desktop', data_binding: 'agent.session.desktop', grid_area: 'main' },
      { id: 'session.comms', kind: 'agent_comms', title: 'Agent Comms', data_binding: 'agent.session.comms', grid_area: 'side' },
      { id: 'session.stream', kind: 'data_stream', title: 'Activity Stream', data_binding: 'agent.session.events', grid_area: 'top' },
    ],
  },
]

function createWidget(slot: string, sourceZoneId: string | undefined, index: number): BoardroomSurfaceWidget {
  return {
    id: `${slot}.widget_${index + 1}`,
    kind: 'metric_strip',
    title: `Widget ${index + 1}`,
    data_binding: sourceZoneId ? `${sourceZoneId}.summary` : slot,
    grid_area: index === 0 ? 'top' : index === 1 ? 'main' : 'side',
  }
}

function applyServicePreset(layout: BoardroomSurfaceLayout, preset: typeof SURFACE_SERVICE_PRESETS[number]): BoardroomSurfaceLayout {
  return {
    ...layout,
    adapter_type: preset.adapterType,
    preview: {
      ...layout.preview,
      mode: preset.previewMode,
      refresh_ms: 5000,
      widgets: preset.widgets,
    },
    focus: {
      ...layout.focus,
      mode: 'native_window',
      target: preset.sourceZoneId,
      refresh_ms: 5000,
    },
    embed: {
      url: preset.embedUrl,
      allow_inline: false,
    },
  }
}

export default function SettingsModule({
  theme,
  editMode,
  viewMode,
  themeOptions,
  monitorAssignments,
  roleAssignmentProfiles = [],
  worldSurfaceAssignments = [],
  futureDomains,
  configWalkthrough,
  rootPath,
  onConfigApplied,
  onUpdateBoardroomAssignment,
  onUpdateSurfaceLayout,
  onUpdateWorldSurfaceLayout,
  onToggleEditMode,
}: SettingsModuleProps) {
  const profiles = useMemo(
    () => getConfigProfiles(configWalkthrough) as ConfigProfile[],
    [configWalkthrough],
  )
  const dimensions = useMemo(
    () => asArray(configWalkthrough?.dimensions).map(toDimension).filter((dimension): dimension is ConfigDimension => dimension !== null),
    [configWalkthrough],
  )
  const activeProfileId = getString(configWalkthrough?.active_profile_id, profiles[0]?.id ?? '')
  const [selectedProfileId, setSelectedProfileId] = useState(activeProfileId)
  const [projectionPreview, setProjectionPreview] = useState<ConfigProjectionPreview | null>(null)
  const [configActionStatus, setConfigActionStatus] = useState<string>('')
  const selectedProfile = profiles.find((profile) => profile.id === selectedProfileId) ?? profiles[0] ?? null
  const canonicalFiles = asRecord(configWalkthrough?.canonical_files) ?? {}

  async function handlePreviewProfile() {
    if (!rootPath || !selectedProfile) {
      setConfigActionStatus('Core state bundle not loaded.')
      return
    }
    try {
      const preview = await previewConfigProfile(rootPath, configWalkthrough, selectedProfile.id)
      setProjectionPreview(preview)
      setConfigActionStatus(`Preview ready: ${preview.changes.filter((change) => change.changed).length} changed targets.`)
    } catch (error) {
      setConfigActionStatus(String(error))
    }
  }

  async function handleApplyProfile() {
    if (!rootPath || !selectedProfile) {
      setConfigActionStatus('Core state bundle not loaded.')
      return
    }
    const preview = projectionPreview ?? await previewConfigProfile(rootPath, configWalkthrough, selectedProfile.id)
    const approved = window.confirm(`Apply ${preview.profileTitle} to ${preview.changes.length} scoped config files?`)
    if (!approved) {
      setConfigActionStatus('Apply cancelled.')
      return
    }
    try {
      await applyConfigProfile(rootPath, preview)
      setProjectionPreview(preview)
      setConfigActionStatus(`Applied ${preview.profileTitle}.`)
      onConfigApplied?.()
    } catch (error) {
      setConfigActionStatus(String(error))
    }
  }

  return (
    <ModuleCard title="Settings" eyebrow="Control surface" accent="cyan">
      <div className="split-stack">
        {profiles.length > 0 && selectedProfile ? (
          <div className="config-walkthrough">
            <div className="config-walkthrough__header">
              <div>
                <div className="module-subtitle">Guided Config Profiles</div>
                <p>{getString(configWalkthrough?.purpose, 'Profile-driven setup flows will appear here.')}</p>
              </div>
              <span className="config-walkthrough__schema">{getString(configWalkthrough?.schema_version, 'unversioned')}</span>
            </div>

            <div className="config-profile-tabs" role="tablist" aria-label="Config profiles">
              {profiles.map((profile) => (
                <button
                  type="button"
                  key={profile.id}
                  className={profile.id === selectedProfile.id ? 'config-profile-tab config-profile-tab--active' : 'config-profile-tab'}
                  onClick={() => setSelectedProfileId(profile.id)}
                >
                  <span>{profile.glyph}</span>
                  {profile.title}
                </button>
              ))}
            </div>

            <section className="config-profile-panel">
              <div className="config-profile-panel__summary">
                <strong>{selectedProfile.productPosture}</strong>
                <p>{selectedProfile.summary}</p>
              </div>
              <div className="config-dimension-grid">
                {dimensions.map((dimension) => {
                  const Icon = DIMENSION_ICONS[dimension.id as keyof typeof DIMENSION_ICONS] ?? SlidersHorizontal
                  return (
                    <article className="config-dimension" key={dimension.id}>
                      <Icon size={17} aria-hidden="true" />
                      <div>
                        <strong>{dimension.label}</strong>
                        <p>{formatSettingLabel(String(selectedProfile.settings[dimension.id] ?? 'unset'))}</p>
                        <span>{dimension.canonicalHome}</span>
                      </div>
                    </article>
                  )
                })}
              </div>
              <div className="config-write-list">
                <div className="module-subtitle">Apply Targets</div>
                {selectedProfile.writes.map((write) => (
                  <span key={write}>{write}</span>
                ))}
              </div>
              <div className="config-action-row">
                <button type="button" className="refresh-button" onClick={handlePreviewProfile}>
                  Preview Diff
                </button>
                <button type="button" className="refresh-button refresh-button--active" onClick={handleApplyProfile}>
                  Apply Profile
                </button>
                {configActionStatus ? <span>{configActionStatus}</span> : null}
              </div>
              {projectionPreview ? (
                <div className="config-preview-list">
                  <div className="module-subtitle">Projection Diff</div>
                  {projectionPreview.changes.map((change) => (
                    <article className="config-preview-list__item" key={change.path}>
                      <strong>{change.path}</strong>
                      <span>{change.changed ? change.summary : 'no change'}</span>
                      <p>{change.afterPreview}</p>
                    </article>
                  ))}
                </div>
              ) : null}
              <div className="document-list compact">
                <article className="document-list__item">
                  <strong>Next Apply Step</strong>
                  <p>{selectedProfile.nextAction}</p>
                </article>
              </div>
            </section>
          </div>
        ) : null}

        <div className="document-list compact">
          <article className="document-list__item">
            <strong>Theme</strong>
            <p>{theme}</p>
          </article>
          <article className="document-list__item">
            <strong>Edit Mode</strong>
            <p>{editMode ? 'enabled' : 'disabled'}</p>
            <button
              type="button"
              className={editMode ? 'refresh-button refresh-button--active' : 'refresh-button'}
              onClick={onToggleEditMode}
            >
              {editMode ? 'Exit Edit Slots' : 'Edit Slots'}
            </button>
          </article>
          <article className="document-list__item">
            <strong>View Mode</strong>
            <p>{viewMode}</p>
          </article>
          <article className="document-list__item">
            <strong>Theme Family</strong>
            <p>{themeOptions.map((option) => option.label).join(' / ')}</p>
          </article>
        </div>
        <div>
          <div className="module-subtitle">Boardroom Surface Slots</div>
          <div className="document-list compact">
            {monitorAssignments.map((assignment) => {
              const layout = assignment.surfaceLayout
              const updateLayout = (updater: BoardroomSurfaceLayout | ((current: BoardroomSurfaceLayout) => BoardroomSurfaceLayout)) => {
                onUpdateSurfaceLayout?.(assignment.slot, updater)
              }

              return (
                <article className="document-list__item boardroom-surface-editor" key={assignment.slot}>
                  <strong>{assignment.label}</strong>
                  <p>{assignment.componentId}</p>
                  <span>{assignment.sourceZoneId ?? 'unassigned'}</span>
                  <select
                    value={assignment.sourceZoneId ?? ''}
                    disabled={!onUpdateBoardroomAssignment}
                    onChange={(event) => onUpdateBoardroomAssignment?.(assignment.slot, event.target.value)}
                    aria-label={`Assignment for ${assignment.slot}`}
                  >
                    <option value="">Unassigned</option>
                    {roleAssignmentProfiles?.map((profile) => (
                      <option key={profile.role_id} value={profile.source_zone_id}>{profile.label}</option>
                    ))}
                  </select>
                      <span>{assignment.role ? `role:${assignment.role}` : 'surface'} / {assignment.adapterType ?? 'adapter'} / {assignment.previewMode ?? 'preview'} / {assignment.focusMode ?? 'focus'}</span>
                  <span>{assignment.widgetCount ?? 0} widgets / {assignment.refreshMs ?? 0}ms preview</span>
                  {assignment.embedUrl ? <span>Embed: {assignment.embedUrl}</span> : null}
                  {assignment.componentId ? <span>Component: {assignment.componentId}</span> : null}
                  {layout && onUpdateSurfaceLayout ? (
                    <div className="boardroom-surface-editor__controls">
                      <label>
                        Adapter
                        <select
                          value={layout.adapter_type}
                          onChange={(event) => updateLayout((current) => ({ ...current, adapter_type: event.target.value as BoardroomSurfaceAdapterType }))}
                        >
                          {SURFACE_ADAPTER_TYPES.map((adapterType) => (
                            <option key={adapterType} value={adapterType}>{formatSettingLabel(adapterType)}</option>
                          ))}
                        </select>
                      </label>
                      <label>
                        Focus
                        <select
                          value={layout.focus.mode}
                          onChange={(event) => updateLayout((current) => ({ ...current, focus: { ...current.focus, mode: event.target.value as BoardroomSurfaceFocusMode } }))}
                        >
                          {SURFACE_FOCUS_MODES.map((focusMode) => (
                            <option key={focusMode} value={focusMode}>{formatSettingLabel(focusMode)}</option>
                          ))}
                        </select>
                      </label>
                      <label>
                        Embed URL
                        <input
                          type="url"
                          value={layout.embed.url ?? ''}
                          placeholder="http://127.0.0.1:3000"
                          onChange={(event) => updateLayout((current) => ({ ...current, embed: { ...current.embed, url: event.target.value.trim() || null } }))}
                        />
                      </label>
                      <label>
                        Refresh MS
                        <input
                          type="number"
                          min={500}
                          step={500}
                          value={layout.preview.refresh_ms}
                          onChange={(event) => {
                            const refreshMs = Number(event.target.value)
                            updateLayout((current) => ({
                              ...current,
                              preview: {
                                ...current.preview,
                                refresh_ms: Number.isFinite(refreshMs) ? refreshMs : current.preview.refresh_ms,
                              },
                            }))
                          }}
                        />
                      </label>
                      <label>
                        Service Preset
                        <select
                          value=""
                          onChange={(event) => {
                            const preset = SURFACE_SERVICE_PRESETS.find((candidate) => candidate.id === event.target.value)
                            if (!preset) return
                            updateLayout((current) => applyServicePreset(current, preset))
                          }}
                        >
                          <option value="">Choose preset</option>
                          {SURFACE_SERVICE_PRESETS.map((preset) => (
                            <option key={preset.id} value={preset.id}>{preset.label}</option>
                          ))}
                        </select>
                      </label>
                      <label className="boardroom-surface-editor__check">
                        <input
                          type="checkbox"
                          checked={layout.embed.allow_inline}
                          onChange={(event) => updateLayout((current) => ({ ...current, embed: { ...current.embed, allow_inline: event.target.checked } }))}
                        />
                        Allow inline embed
                      </label>
                      <div className="boardroom-surface-editor__widgets">
                        <div className="boardroom-surface-editor__widgets-header">
                          <span>Preview Widgets</span>
                          <button
                            type="button"
                            className="refresh-button"
                            onClick={() => updateLayout((current) => ({
                              ...current,
                              preview: {
                                ...current.preview,
                                widgets: [...current.preview.widgets, createWidget(assignment.slot, assignment.sourceZoneId, current.preview.widgets.length)],
                              },
                            }))}
                          >
                            Add Widget
                          </button>
                        </div>
                        {layout.preview.widgets.map((widget, widgetIndex) => (
                          <div className="boardroom-surface-editor__widget" key={widget.id}>
                            <label>
                              Kind
                              <select
                                value={widget.kind}
                                onChange={(event) => updateLayout((current) => ({
                                  ...current,
                                  preview: {
                                    ...current.preview,
                                    widgets: current.preview.widgets.map((currentWidget, index) => index === widgetIndex
                                      ? { ...currentWidget, kind: event.target.value as BoardroomSurfaceWidgetKind }
                                      : currentWidget),
                                  },
                                }))}
                              >
                                {SURFACE_WIDGET_KINDS.map((widgetKind) => (
                                  <option key={widgetKind} value={widgetKind}>{formatSettingLabel(widgetKind)}</option>
                                ))}
                              </select>
                            </label>
                            <label>
                              Title
                              <input
                                type="text"
                                value={widget.title}
                                onChange={(event) => updateLayout((current) => ({
                                  ...current,
                                  preview: {
                                    ...current.preview,
                                    widgets: current.preview.widgets.map((currentWidget, index) => index === widgetIndex
                                      ? { ...currentWidget, title: event.target.value }
                                      : currentWidget),
                                  },
                                }))}
                              />
                            </label>
                            <label>
                              Binding
                              <input
                                type="text"
                                value={widget.data_binding}
                                onChange={(event) => updateLayout((current) => ({
                                  ...current,
                                  preview: {
                                    ...current.preview,
                                    widgets: current.preview.widgets.map((currentWidget, index) => index === widgetIndex
                                      ? { ...currentWidget, data_binding: event.target.value }
                                      : currentWidget),
                                  },
                                }))}
                              />
                            </label>
                            <label>
                              Grid Area
                              <input
                                type="text"
                                value={widget.grid_area}
                                onChange={(event) => updateLayout((current) => ({
                                  ...current,
                                  preview: {
                                    ...current.preview,
                                    widgets: current.preview.widgets.map((currentWidget, index) => index === widgetIndex
                                      ? { ...currentWidget, grid_area: event.target.value }
                                      : currentWidget),
                                  },
                                }))}
                              />
                            </label>
                            <button
                              type="button"
                              className="refresh-button"
                              onClick={() => updateLayout((current) => ({
                                ...current,
                                preview: {
                                  ...current.preview,
                                  widgets: current.preview.widgets.filter((_, index) => index !== widgetIndex),
                                },
                              }))}
                            >
                              Remove
                            </button>
                          </div>
                        ))}
                      </div>
                    </div>
                  ) : null}
                </article>
              )
            })}
          </div>
        </div>
        {worldSurfaceAssignments.length > 0 ? (
          <div>
            <div className="module-subtitle">World Surface Slots</div>
            <div className="document-list compact">
              {worldSurfaceAssignments.map((assignment) => {
                const layout = assignment.surfaceLayout
                const updateLayout = (updater: BoardroomSurfaceLayout | ((current: BoardroomSurfaceLayout) => BoardroomSurfaceLayout)) => {
                  onUpdateWorldSurfaceLayout?.(assignment.slot, updater)
                }

                return (
                  <article className="document-list__item boardroom-surface-editor" key={assignment.slot}>
                    <strong>{assignment.slot}</strong>
                    <p>{assignment.label}</p>
                    <span>{assignment.sourceZoneId ?? 'unassigned'}</span>
                    <span>{assignment.role ?? 'world_surface'} / {assignment.adapterType ?? 'adapter'} / {assignment.previewMode ?? 'preview'} / {assignment.focusMode ?? 'focus'}</span>
                    <span>{assignment.widgetCount ?? 0} widgets / {assignment.refreshMs ?? 0}ms preview</span>
                    {assignment.componentId ? <span>{assignment.componentId}</span> : null}
                    {layout && onUpdateWorldSurfaceLayout ? (
                      <div className="boardroom-surface-editor__controls">
                        <label>
                          Adapter
                          <select
                            value={layout.adapter_type}
                            onChange={(event) => updateLayout((current) => ({ ...current, adapter_type: event.target.value as BoardroomSurfaceAdapterType }))}
                          >
                            {SURFACE_ADAPTER_TYPES.map((adapterType) => (
                              <option key={adapterType} value={adapterType}>{formatSettingLabel(adapterType)}</option>
                            ))}
                          </select>
                        </label>
                        <label>
                          Focus
                          <select
                            value={layout.focus.mode}
                            onChange={(event) => updateLayout((current) => ({ ...current, focus: { ...current.focus, mode: event.target.value as BoardroomSurfaceFocusMode } }))}
                          >
                            {SURFACE_FOCUS_MODES.map((focusMode) => (
                              <option key={focusMode} value={focusMode}>{formatSettingLabel(focusMode)}</option>
                            ))}
                          </select>
                        </label>
                        <label>
                          Focus Target
                          <input
                            type="text"
                            value={layout.focus.target}
                            onChange={(event) => updateLayout((current) => ({ ...current, focus: { ...current.focus, target: event.target.value.trim() || current.focus.target } }))}
                          />
                        </label>
                        <label>
                          Refresh MS
                          <input
                            type="number"
                            min={500}
                            step={500}
                            value={layout.preview.refresh_ms}
                            onChange={(event) => {
                              const refreshMs = Number(event.target.value)
                              updateLayout((current) => ({
                                ...current,
                                preview: {
                                  ...current.preview,
                                  refresh_ms: Number.isFinite(refreshMs) ? refreshMs : current.preview.refresh_ms,
                                },
                              }))
                            }}
                          />
                        </label>
                      </div>
                    ) : null}
                  </article>
                )
              })}
            </div>
          </div>
        ) : null}
        <div>
          <div className="module-subtitle">Canonical Config Homes</div>
          <div className="document-list compact">
            {Object.entries(canonicalFiles).slice(0, 8).map(([label, path]) => (
              <article className="document-list__item" key={label}>
                <strong>{formatSettingLabel(label)}</strong>
                <p>{getString(path)}</p>
              </article>
            ))}
          </div>
        </div>
        <div>
          <div className="module-subtitle">Reserved Domain Modules</div>
          <div className="document-list compact">
            {futureDomains.map((domain) => (
              <article className="document-list__item" key={domain.title}>
                <strong>{domain.title}</strong>
                <p>{domain.status}</p>
              </article>
            ))}
          </div>
        </div>
      </div>
    </ModuleCard>
  )
}
