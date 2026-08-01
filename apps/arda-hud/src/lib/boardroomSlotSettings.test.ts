// sigil: REPAIR
import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  ARDA_BOARDROOM_SLOT_SETTINGS_RELATIVE_PATH,
  BOARDROOM_WORKSTATION_ROLE_PROFILES,
  BOARDROOM_SCENE_SLOT_IDS,
  DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS,
  assignmentsFromDocument,
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
  resetBoardroomProfile,
  saveBoardroomSlotSettings,
  saveBoardroomSlotSettingsDocument,
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
})
