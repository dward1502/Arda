// sigil: REPAIR
import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH,
  BOARDROOM_WORKSTATION_ROLE_PROFILES,
  BOARDROOM_SCENE_SLOT_IDS,
  DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS,
  assignmentsFromDocument,
  claimMonitorSlot,
  createDefaultBoardroomSlotSettings,
  documentFromAssignments,
  documentWithSurfaceLayout,
  documentWithVisualizationSelection,
  exportBoardroomProfile,
  importBoardroomProfile,
  loadBoardroomSlotSettings,
  parseBoardroomSlotSettings,
  readLocalBoardroomSlotAssignments,
  readLocalBoardroomSlotSettingsDocument,
  refreshMonitorSlot,
  releaseMonitorSlot,
  resetBoardroomProfile,
  resetMonitorSlot,
  saveBoardroomSlotSettings,
  saveBoardroomSlotSettingsDocument,
  resolveMonitorSlotSource,
  type BoardroomAgentClaim,
} from './boardroomSlotSettings'
import { readFile, writeScopedFile } from './weathertop'

vi.mock('./weathertop', () => ({
  readFile: vi.fn(),
  writeScopedFile: vi.fn(),
}))

const mockedReadFile = vi.mocked(readFile)
const mockedWriteScopedFile = vi.mocked(writeScopedFile)

describe('boardroom slot settings', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('creates a complete slot contract document with stable scene slot ids', () => {
    const document = createDefaultBoardroomSlotSettings('2026-05-22T00:00:00.000Z')

    expect(document.schema_version).toBe('arda.arda_boardroom_slots.v1')
    expect(document.authority).toBe(ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH)
    expect(document.assignments.map((assignment) => assignment.slot_id)).toEqual([...BOARDROOM_SCENE_SLOT_IDS])
    expect(document.assignments[0]).toMatchObject({
      slot_id: 'monitor_left_1',
      component_id: 'warp-dev-service-surface',
      source_zone_id: 'service_warp_dev',
      module_ids: ['service_embed'],
      surface_layout: {
        adapter_type: 'external_url',
        preview: {
          mode: 'service_status',
        },
        focus: {
          mode: 'native_window',
          target: 'service_warp_dev',
        },
      },
      visualization: {
        preset_id: 'standby',
        config: {
          density: 'medium',
          timespan_minutes: 15,
          alert_threshold: null,
        },
      },
    })
  })

  it('applies compatible visualization selections and retains the last valid selection on incompatibility', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const accepted = documentWithVisualizationSelection(document, 'monitor_left_2', {
      preset_id: 'topology',
      config: { density: 'high', timespan_minutes: 30, alert_threshold: 0.75 },
    }, '2026-07-30T12:01:00.000Z')
    expect(accepted.ok).toBe(true)
    expect(accepted.document.assignments.find((assignment) => assignment.slot_id === 'monitor_left_2')?.visualization).toMatchObject({
      preset_id: 'topology',
      config: { density: 'high', timespan_minutes: 30, alert_threshold: 0.75 },
    })

    const rejected = documentWithVisualizationSelection(accepted.document, 'monitor_left_2', {
      preset_id: 'constellation',
      config: { density: 'low', timespan_minutes: 60, alert_threshold: null },
    }, '2026-07-30T12:02:00.000Z')
    expect(rejected.ok).toBe(false)
    expect(rejected.document).toBe(accepted.document)
    expect(rejected.message).toContain('retained Topology')
  })

  it('exports, imports, and resets complete recoverable boardroom profiles', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T13:00:00.000Z')
    const exported = exportBoardroomProfile(document)
    const imported = importBoardroomProfile(exported)

    expect(imported.ok).toBe(true)
    expect(imported.document).toEqual(document)
    expect(importBoardroomProfile('{broken')).toMatchObject({ ok: false, document: null })
    expect(resetBoardroomProfile('2026-07-30T14:00:00.000Z')).toEqual(
      createDefaultBoardroomSlotSettings('2026-07-30T14:00:00.000Z'),
    )
  })

  it('loads a complete local profile while remaining backward compatible with assignment-only storage', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T15:00:00.000Z')
    const documentStorage = { getItem: () => exportBoardroomProfile(document) }
    expect(readLocalBoardroomSlotSettingsDocument(documentStorage)).toEqual(document)
    expect(readLocalBoardroomSlotAssignments(documentStorage)).toEqual(assignmentsFromDocument(document))

    const legacyStorage = { getItem: () => JSON.stringify({ monitor_left_1: 'custom_zone' }) }
    expect(readLocalBoardroomSlotAssignments(legacyStorage).monitor_left_1).toBe('custom_zone')
  })

  it('normalizes partial workspace documents without losing local placeholders', () => {
    const parsed = parseBoardroomSlotSettings({
      schema_version: 'arda.arda_boardroom_slots.v1',
      updated_at_utc: '2026-05-22T01:00:00.000Z',
      assignments: [
        {
          slot_id: 'monitor_left_2',
          component_id: 'custom-routing',
          source_zone_id: 'routing_and_comms',
          title: 'Routing',
          module_ids: ['operations_and_packages'],
          presentation_modes: ['in_scene'],
          surface_layout: {
            adapter_type: 'component_grid',
            preview: {
              mode: 'component_grid',
              refresh_ms: 1234,
              widgets: [
                {
                  id: 'routing.flow',
                  kind: 'particle_stream',
                  title: 'Routing flow',
                  data_binding: 'routing.health',
                  grid_area: 'main',
                },
              ],
            },
            focus: {
              mode: 'native_window',
              target: 'routing_and_comms',
              refresh_ms: 1000,
            },
            embed: {
              url: null,
              allow_inline: false,
            },
          },
          updated_at_utc: '2026-05-22T01:00:00.000Z',
        },
        {
          slot_id: 'not_a_scene_slot',
          source_zone_id: 'discarded',
        },
      ],
    })

    expect(parsed).not.toBeNull()
    expect(parsed?.assignments).toHaveLength(BOARDROOM_SCENE_SLOT_IDS.length)
    expect(assignmentsFromDocument(parsed!).monitor_left_2).toBe('routing_and_comms')
    expect(parsed?.assignments.find((assignment) => assignment.slot_id === 'monitor_left_2')?.surface_layout.preview.widgets[0]).toMatchObject({
      id: 'routing.flow',
      kind: 'particle_stream',
    })
    expect(assignmentsFromDocument(parsed!).view_desk_aux).toBe(DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS.view_desk_aux)
  })

  it('reads browser-local assignments defensively', () => {
    const storage = {
      getItem: () => JSON.stringify({
        monitor_left_1: 'custom_zone',
        view_desk_l: 42,
      }),
    }

    expect(readLocalBoardroomSlotAssignments(storage).monitor_left_1).toBe('custom_zone')
    expect(readLocalBoardroomSlotAssignments(storage).view_desk_l).toBe(DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS.view_desk_l)
    expect(readLocalBoardroomSlotAssignments({ getItem: () => '{broken' })).toEqual(DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS)
  })

  it('loads workspace assignments when the core state file is available', async () => {
    mockedReadFile.mockResolvedValueOnce({
      success: true,
      content: JSON.stringify(documentFromAssignments({
        ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS,
        monitor_left_1: 'governance_guardhouse',
      }, '2026-05-22T02:00:00.000Z')),
      error: null,
      path: ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH,
    })

    const result = await loadBoardroomSlotSettings('/arda')

    expect(mockedReadFile).toHaveBeenCalledWith(`/arda/${ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH}`)
    expect(result.mode).toBe('workspace')
    expect(result.assignments.monitor_left_1).toBe('governance_guardhouse')
  })

  it('saves assignments through the scoped write IPC contract only', async () => {
    mockedWriteScopedFile.mockResolvedValueOnce({ success: true, content: 'ok', error: null, path: ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH })

    const result = await saveBoardroomSlotSettings('/arda', {
      ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS,
      view_desk_aux: 'hermes_runtime',
    })

    expect(result.success).toBe(true)
    expect(mockedWriteScopedFile).toHaveBeenCalledOnce()
    const [rootPath, relativePath, content] = mockedWriteScopedFile.mock.calls[0]
    expect(rootPath).toBe('/arda')
    expect(relativePath).toBe(ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH)
    expect(JSON.parse(content).assignments.find((assignment: { slot_id: string }) => assignment.slot_id === 'view_desk_aux').source_zone_id).toBe('hermes_runtime')
  })

  it('updates and saves a surface layout without dropping the slot contract document', async () => {
    mockedWriteScopedFile.mockResolvedValueOnce({ success: true, content: 'ok', error: null, path: ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH })
    const document = createDefaultBoardroomSlotSettings('2026-06-01T00:00:00.000Z')
    const current = document.assignments.find((assignment) => assignment.slot_id === 'monitor_left_2')!.surface_layout
    const updated = documentWithSurfaceLayout(document, 'monitor_left_2', {
      ...current,
      adapter_type: 'service_embed',
      focus: {
        ...current.focus,
        mode: 'native_window',
      },
      embed: {
        url: 'http://127.0.0.1:3000',
        allow_inline: false,
      },
    }, '2026-06-01T01:00:00.000Z')

    await saveBoardroomSlotSettingsDocument('/arda', updated)

    const [, , content] = mockedWriteScopedFile.mock.calls[0]
    const saved = JSON.parse(content)
    expect(saved.assignments).toHaveLength(BOARDROOM_SCENE_SLOT_IDS.length)
    expect(saved.assignments.find((assignment: { slot_id: string }) => assignment.slot_id === 'monitor_left_2').surface_layout).toMatchObject({
      adapter_type: 'service_embed',
      embed: {
        url: 'http://127.0.0.1:3000',
        allow_inline: false,
      },
    })
  })

  it('creates safe native-window layouts for configured Beelink local services', () => {
    const document = documentFromAssignments({
      ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS,
      monitor_left_1: 'service_beelink_grafana',
      monitor_left_2: 'service_beelink_openwebui',
    }, '2026-06-01T02:00:00.000Z')

    expect(document.assignments.find((assignment) => assignment.slot_id === 'monitor_left_1')?.surface_layout).toMatchObject({
      adapter_type: 'service_embed',
      focus: {
        mode: 'native_window',
        target: 'service_beelink_grafana',
      },
      embed: {
        url: 'http://100.103.125.88:3000',
        allow_inline: false,
      },
    })
    expect(document.assignments.find((assignment) => assignment.slot_id === 'monitor_left_2')?.surface_layout.embed).toMatchObject({
      url: 'http://100.103.125.88:8080',
      allow_inline: false,
    })
  })

  it('derives Fleet assignment metadata from the role profile for any physical slot', () => {
    const fleetProfile = BOARDROOM_WORKSTATION_ROLE_PROFILES.find((profile) => profile.role_id === 'fleet')!
    const document = documentFromAssignments({
      ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS,
      monitor_left_1: fleetProfile.source_zone_id,
    }, '2026-06-01T03:00:00.000Z')
    const assignment = document.assignments.find((candidate) => candidate.slot_id === 'monitor_left_1')!

    expect(assignment).toMatchObject({
      slot_id: 'monitor_left_1',
      role_id: 'fleet',
      source_zone_id: 'systems_health',
      component_id: 'fleet-workstation',
      title: 'Fleet',
      module_ids: ['systems', 'operations_and_packages'],
    })
    expect(assignment.surface_layout.focus.target).toBe('systems_health')
  })

  it('normalizes role-only assignment documents for backward-compatible saves', () => {
    const parsed = parseBoardroomSlotSettings({
      schema_version: 'arda.arda_boardroom_slots.v1',
      updated_at_utc: '2026-06-01T04:00:00.000Z',
      assignments: [
        {
          slot_id: 'monitor_left_1',
          role_id: 'fleet',
          updated_at_utc: '2026-06-01T04:00:00.000Z',
        },
      ],
    })

    const assignment = parsed?.assignments.find((candidate) => candidate.slot_id === 'monitor_left_1')
    expect(assignment).toMatchObject({
      role_id: 'fleet',
      source_zone_id: 'systems_health',
      component_id: 'fleet-workstation',
      module_ids: ['systems', 'operations_and_packages'],
    })
  })

  it('degrades malformed surface layout fields into safe defaults', () => {
    const malformed = parseBoardroomSlotSettings({
      schema_version: 'arda.arda_boardroom_slots.v1',
      updated_at_utc: '2026-07-30T12:00:00.000Z',
      assignments: [
        {
          slot_id: 'monitor_left_1',
          component_id: 'test-surface',
          source_zone_id: 'service_test',
          title: 'Malformed Test',
          module_ids: ['service_embed'],
          presentation_modes: ['in_scene'],
          surface_layout: {
            // unknown adapter_type falls back to component_grid via default
            adapter_type: 'nonexistent_adapter',
            preview: {
              mode: 'unknown_preview_mode',
              refresh_ms: 'not-a-number',
              widgets: [
                // unknown widget kind degrades to metric_strip
                { id: 'w1', kind: 'unknown_kind', title: 'Bad', data_binding: 'x', grid_area: 'main' },
                // widget with missing kind
                { id: 'w2', title: 'NoKind', data_binding: 'y', grid_area: 'side' },
              ],
            },
            focus: {
              mode: 'bogus_focus_mode',
              target: 12345,
              refresh_ms: null,
            },
            embed: {
              url: null,
              allow_inline: 'yes',
            },
          },
          updated_at_utc: '2026-07-30T12:00:00.000Z',
        },
      ],
    })

    expect(malformed).not.toBeNull()
    const layout = malformed!.assignments[0].surface_layout
    // unknown adapter_type is preserved as string but parseSurfaceLayout keeps it
    expect(layout.adapter_type).toBe('nonexistent_adapter')
    expect(layout.preview.mode).toBe('unknown_preview_mode')
    // non-finite refresh_ms falls back to default
    expect(layout.preview.refresh_ms).toBeGreaterThan(0)
    // unknown widget kind falls back to the default layout's widget at that index
    // (service_test → status_grid at index 0); missing kind too
    expect(layout.preview.widgets[0].kind).toBe('status_grid')
    expect(layout.preview.widgets[1].kind).toBe('metric_strip')
    // widget ids and data_binding preserved
    expect(layout.preview.widgets[0].id).toBe('w1')
    expect(layout.preview.widgets[0].data_binding).toBe('x')
    expect(layout.preview.widgets[1].id).toBe('w2')
    expect(layout.preview.widgets[1].data_binding).toBe('y')
    // non-string focus.target falls back; non-string mode preserved as-is by parser
    expect(typeof layout.focus.target).toBe('string')
    expect(layout.focus.refresh_ms).toBeGreaterThan(0)
    // non-boolean allow_inline falls back to boolean default
    expect(typeof layout.embed.allow_inline).toBe('boolean')
  })

  it('preserves remote_preview focus mode through round-trip', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const updated = documentWithSurfaceLayout(document, 'monitor_left_1', {
      ...document.assignments[0].surface_layout,
      focus: {
        mode: 'remote_preview',
        target: 'service_warp_dev',
        refresh_ms: 2000,
      },
    }, '2026-07-30T12:01:00.000Z')
    const exported = exportBoardroomProfile(updated)
    const imported = importBoardroomProfile(exported)
    expect(imported.ok).toBe(true)
    const layout = imported.document!.assignments[0].surface_layout
    expect(layout.focus.mode).toBe('remote_preview')
    expect(layout.focus.target).toBe('service_warp_dev')
  })

  it('claims a monitor slot and resolves the live binding at runtime', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const claim: BoardroomAgentClaim = {
      owner: 'hermes-agent-001',
      activity_kind: 'agent_activity',
      payload_binding: 'hermes.live_stream',
      fallback_preview: document.assignments[0].surface_layout.preview,
      lease_expires_at_utc: '2026-12-31T23:59:59.000Z',
    }
    const claimed = claimMonitorSlot(document, 'monitor_left_1', claim, '2026-07-30T12:01:00.000Z')
    const monitorAssignment = claimed.assignments.find((a) => a.slot_id === 'monitor_left_1')!
    expect(monitorAssignment.agent_claims).toHaveLength(1)
    expect(monitorAssignment.agent_claims![0].owner).toBe('hermes-agent-001')

    const resolved = resolveMonitorSlotSource('monitor_left_1', claimed, '2026-07-30T12:02:00.000Z')
    expect(resolved).not.toBeNull()
    expect(resolved!.active).toBe(true)
    expect(resolved!.claim?.owner).toBe('hermes-agent-001')
  })

  it('falls back to persisted assignment when no live claim is active', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const resolved = resolveMonitorSlotSource('monitor_left_1', document, '2026-07-30T12:00:00.000Z')
    expect(resolved).not.toBeNull()
    expect(resolved!.active).toBe(false)
    expect(resolved!.claim).toBeNull()
    expect(resolved!.sourceZoneId).toBe('service_warp_dev')
  })

  it('does not resolve claims for non-monitor (desk) slots', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const resolved = resolveMonitorSlotSource('view_desk_l', document, '2026-07-30T12:00:00.000Z')
    expect(resolved).toBeNull()
  })

  it('releases a monitor claim for a single owner without clearing others', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const claimA: BoardroomAgentClaim = {
      owner: 'hermes-agent-001',
      activity_kind: 'agent_activity',
      payload_binding: 'hermes.live_stream',
      fallback_preview: document.assignments[0].surface_layout.preview,
      lease_expires_at_utc: '2026-12-31T23:59:59.000Z',
    }
    const claimB: BoardroomAgentClaim = {
      owner: 'hermes-agent-002',
      activity_kind: 'streaming_text',
      payload_binding: 'queue.heartbeat',
      fallback_preview: document.assignments[0].surface_layout.preview,
      lease_expires_at_utc: '2026-12-31T23:59:59.000Z',
    }
    const withTwo = claimMonitorSlot(claimMonitorSlot(document, 'monitor_left_1', claimA), 'monitor_left_1', claimB)
    expect(withTwo.assignments[0].agent_claims).toHaveLength(2)

    const released = releaseMonitorSlot(withTwo, 'monitor_left_1', 'hermes-agent-001', '2026-07-30T12:05:00.000Z')
    const releasedClaims = released.assignments[0].agent_claims ?? []
    expect(releasedClaims).toHaveLength(1)
    expect(releasedClaims[0].owner).toBe('hermes-agent-002')
    // other slots are untouched
    expect(released.assignments.find((a) => a.slot_id === 'monitor_left_2')?.agent_claims).toBeUndefined()
  })

  it('refreshes only the matching monitor owner lease', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const claim: BoardroomAgentClaim = {
      owner: 'hermes-agent-001',
      activity_kind: 'agent_activity',
      payload_binding: 'hermes.live_stream',
      fallback_preview: document.assignments[0].surface_layout.preview,
      lease_expires_at_utc: '2026-07-30T12:05:00.000Z',
    }
    const claimed = claimMonitorSlot(document, 'monitor_left_1', claim)

    const refreshed = refreshMonitorSlot(
      claimed,
      'monitor_left_1',
      'hermes-agent-001',
      '2026-07-30T12:10:00.000Z',
      '2026-07-30T12:06:00.000Z',
    )

    expect(refreshed.assignments[0].agent_claims?.[0].lease_expires_at_utc).toBe('2026-07-30T12:10:00.000Z')
    expect(refreshed.assignments[0].updated_at_utc).toBe('2026-07-30T12:06:00.000Z')
    expect(() => refreshMonitorSlot(claimed, 'monitor_left_1', 'other-agent', '2026-07-30T12:10:00.000Z')).toThrow(/does not own/)
  })

  it('resets a monitor slot claim and restores default surface layout', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const claim: BoardroomAgentClaim = {
      owner: 'hermes-agent-001',
      activity_kind: 'agent_activity',
      payload_binding: 'hermes.live_stream',
      fallback_preview: document.assignments[0].surface_layout.preview,
      lease_expires_at_utc: '2026-12-31T23:59:59.000Z',
    }
    const claimed = claimMonitorSlot(document, 'monitor_left_1', claim, '2026-07-30T12:01:00.000Z')
    expect(claimed.assignments[0].agent_claims).toBeDefined()

    const reset = resetMonitorSlot(claimed, 'monitor_left_1', '2026-07-30T12:10:00.000Z')
    expect(reset.assignments[0].agent_claims).toBeUndefined()
    expect(reset.assignments[0].surface_layout).toEqual(
      createDefaultBoardroomSlotSettings('2026-07-30T12:10:00.000Z').assignments[0].surface_layout,
    )
    // other slots untouched
    expect(reset.assignments.find((a) => a.slot_id === 'monitor_left_2')?.source_zone_id).toBe('routing_and_comms')
  })

  it('round-trips agent claims through export/import persistence', () => {
    const document = createDefaultBoardroomSlotSettings('2026-07-30T12:00:00.000Z')
    const claim: BoardroomAgentClaim = {
      owner: 'hermes-agent-001',
      activity_kind: 'agent_activity',
      payload_binding: 'hermes.live_stream',
      fallback_preview: document.assignments[0].surface_layout.preview,
      lease_expires_at_utc: '2026-12-31T23:59:59.000Z',
    }
    const claimed = claimMonitorSlot(document, 'monitor_left_1', claim, '2026-07-30T12:01:00.000Z')
    const exported = exportBoardroomProfile(claimed)
    const imported = importBoardroomProfile(exported)
    expect(imported.ok).toBe(true)
    const monitorAssignment = imported.document!.assignments.find((a) => a.slot_id === 'monitor_left_1')!
    expect(monitorAssignment.agent_claims).toHaveLength(1)
    expect(monitorAssignment.agent_claims![0].owner).toBe('hermes-agent-001')
    expect(monitorAssignment.agent_claims![0].activity_kind).toBe('agent_activity')
  })

  it('filters out malformed agent claims during parse', () => {
    const parsed = parseBoardroomSlotSettings({
      schema_version: 'arda.arda_boardroom_slots.v1',
      updated_at_utc: '2026-07-30T12:00:00.000Z',
      assignments: [
        {
          slot_id: 'monitor_left_1',
          component_id: 'warp-dev-service-surface',
          source_zone_id: 'service_warp_dev',
          title: 'Warp',
          module_ids: ['service_embed'],
          presentation_modes: ['in_scene'],
          surface_layout: {
            enabled: true,
            adapter_type: 'external_url',
            preview: { mode: 'service_status', refresh_ms: 5000, widgets: [] },
            focus: { mode: 'native_window', target: 'service_warp_dev', refresh_ms: 5000 },
            embed: { url: null, allow_inline: false },
          },
          agent_claims: [
            { owner: 'good-agent', activity_kind: 'agent_activity', payload_binding: 'x', lease_expires_at_utc: '2026-12-31T23:59:59.000Z', fallback_preview: {} },
            { owner: '', activity_kind: 'streaming_text', payload_binding: 'bad', lease_expires_at_utc: '2026-12-31T23:59:59.000Z', fallback_preview: {} },
            { owner: 'no-lease', activity_kind: 'bad_kind', payload_binding: 'x', fallback_preview: {} },
          ],
          updated_at_utc: '2026-07-30T12:00:00.000Z',
        },
      ],
    })
    expect(parsed).not.toBeNull()
    const claims = parsed!.assignments[0].agent_claims ?? []
    expect(claims).toHaveLength(2)
    expect(claims[0].owner).toBe('good-agent')
    expect(claims[1].activity_kind).toBe('agent_activity')
  })
})
