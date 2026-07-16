// sigil: REPAIR
import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH,
  WORLD_SCENE_SURFACE_IDS,
  createDefaultWorldSurfaceSettings,
  loadWorldSurfaceSettings,
  parseWorldSurfaceSettings,
  readLocalWorldSurfaceAssignments,
  saveWorldSurfaceSettingsDocument,
  worldSurfaceAssignmentsFromDocument,
  worldSurfaceDocumentWithLayout,
} from './worldSurfaceSettings'
import { readFile, writeScopedFile } from './weathertop'

vi.mock('./weathertop', () => ({
  readFile: vi.fn(),
  writeScopedFile: vi.fn(),
}))

const mockedReadFile = vi.mocked(readFile)
const mockedWriteScopedFile = vi.mocked(writeScopedFile)

describe('world surface settings', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('creates a complete world surface document for districts and terminals', () => {
    const document = createDefaultWorldSurfaceSettings('2026-06-01T00:00:00.000Z')

    expect(document.schema_version).toBe('arda.arda_world_surfaces.v1')
    expect(document.authority).toBe(ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH)
    expect(document.assignments.map((assignment) => assignment.surface_id)).toEqual([...WORLD_SCENE_SURFACE_IDS])
    expect(document.assignments.find((assignment) => assignment.surface_id === 'district_operations')).toMatchObject({
      source_zone_id: 'planning_and_queue',
      role: 'district',
      surface_layout: {
        adapter_type: 'component_grid',
        focus: {
          mode: 'in_scene_workstation',
          target: 'planning_and_queue',
        },
      },
    })
    expect(document.assignments.find((assignment) => assignment.surface_id === 'terminal_queue')).toMatchObject({
      source_zone_id: 'planning_and_queue',
      role: 'terminal',
      surface_layout: {
        adapter_type: 'streaming_text',
        preview: {
          mode: 'stream_feed',
        },
      },
    })
  })

  it('normalizes partial workspace documents and rejects unknown surfaces', () => {
    const parsed = parseWorldSurfaceSettings({
      schema_version: 'arda.arda_world_surfaces.v1',
      updated_at_utc: '2026-06-01T01:00:00.000Z',
      assignments: [
        {
          surface_id: 'district_monitoring',
          component_id: 'custom-monitoring',
          source_zone_id: 'systems_health',
          title: 'Fleet Wall',
          role: 'district',
          module_ids: ['systems'],
          presentation_modes: ['in_scene'],
          surface_layout: {
            adapter_type: 'agent_activity',
            preview: {
              mode: 'agent_activity',
              refresh_ms: 2000,
              widgets: [
                {
                  id: 'fleet.pulse',
                  kind: 'particle_stream',
                  title: 'Fleet pulse',
                  data_binding: 'systems_health.pressure',
                  grid_area: 'main',
                },
              ],
            },
            focus: {
              mode: 'native_window',
              target: 'systems_health',
              refresh_ms: 1000,
            },
            embed: {
              url: null,
              allow_inline: false,
            },
          },
          updated_at_utc: '2026-06-01T01:00:00.000Z',
        },
        {
          surface_id: 'unknown_surface',
          source_zone_id: 'discarded',
        },
      ],
    })

    expect(parsed).not.toBeNull()
    expect(parsed?.assignments).toHaveLength(WORLD_SCENE_SURFACE_IDS.length)
    expect(worldSurfaceAssignmentsFromDocument(parsed!).district_monitoring).toBe('systems_health')
    expect(parsed?.assignments.find((assignment) => assignment.surface_id === 'district_monitoring')?.surface_layout.preview.widgets[0]).toMatchObject({
      id: 'fleet.pulse',
      kind: 'particle_stream',
    })
    expect(worldSurfaceAssignmentsFromDocument(parsed!).terminal_status).toBe('sovereign_world')
  })

  it('reads browser-local world surface assignments defensively', () => {
    const storage = {
      getItem: () => JSON.stringify({
        terminal_queue: 'routing_and_comms',
        district_command: 42,
      }),
    }

    expect(readLocalWorldSurfaceAssignments(storage).terminal_queue).toBe('routing_and_comms')
    expect(readLocalWorldSurfaceAssignments(storage).district_command).toBe('sovereign_world')
    expect(readLocalWorldSurfaceAssignments({ getItem: () => '{broken' }).terminal_tools).toBe('systems_health')
  })

  it('loads workspace world surfaces when the core state file is available', async () => {
    mockedReadFile.mockResolvedValueOnce({
      success: true,
      content: JSON.stringify(createDefaultWorldSurfaceSettings('2026-06-01T02:00:00.000Z')),
      error: null,
    })

    const result = await loadWorldSurfaceSettings('/arda')

    expect(mockedReadFile).toHaveBeenCalledWith(`/arda/${ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH}`)
    expect(result.mode).toBe('workspace')
    expect(result.assignments.district_operations).toBe('planning_and_queue')
  })

  it('saves layout edits through scoped write IPC', async () => {
    mockedWriteScopedFile.mockResolvedValueOnce({ success: true, content: 'ok', error: null })
    const document = createDefaultWorldSurfaceSettings('2026-06-01T00:00:00.000Z')
    const current = document.assignments.find((assignment) => assignment.surface_id === 'terminal_tools')!.surface_layout
    const updated = worldSurfaceDocumentWithLayout(document, 'terminal_tools', {
      ...current,
      adapter_type: 'remote_desktop',
      focus: {
        ...current.focus,
        mode: 'native_window',
      },
    }, '2026-06-01T03:00:00.000Z')

    await saveWorldSurfaceSettingsDocument('/arda', updated)

    const [rootPath, relativePath, content] = mockedWriteScopedFile.mock.calls[0]
    expect(rootPath).toBe('/arda')
    expect(relativePath).toBe(ARDA_WORLD_SURFACE_SETTINGS_RELATIVE_PATH)
    expect(JSON.parse(content).assignments.find((assignment: { surface_id: string }) => assignment.surface_id === 'terminal_tools').surface_layout).toMatchObject({
      adapter_type: 'remote_desktop',
      focus: {
        mode: 'native_window',
      },
    })
  })
})
