import { describe, expect, it } from 'vitest'
import {
  BOARDROOM_VISUALIZATION_PRESETS,
  compatibleBoardroomVisualizationPresets,
  resolveBoardroomVisualizationSelection,
} from './boardroomVisualizationPresets'

describe('boardroom visualization presets', () => {
  it('exposes stable presets with explicit compatible input kinds', () => {
    expect(BOARDROOM_VISUALIZATION_PRESETS.map((preset) => preset.id)).toEqual([
      'standby',
      'topology',
      'routes',
      'lanes',
      'constellation',
      'pulse',
    ])
    expect(compatibleBoardroomVisualizationPresets('routing_and_comms').map((preset) => preset.id)).toEqual([
      'standby',
      'topology',
      'routes',
      'pulse',
    ])
  })

  it('resolves a compatible requested preset without changing its configuration', () => {
    expect(resolveBoardroomVisualizationSelection('routing_and_comms', {
      preset_id: 'topology',
      config: { density: 'high', timespan_minutes: 30, alert_threshold: 0.8 },
    })).toEqual({
      ok: true,
      selection: {
        preset_id: 'topology',
        config: { density: 'high', timespan_minutes: 30, alert_threshold: 0.8 },
      },
      message: 'Topology is compatible with routing input',
    })
  })

  it('fails closed to the last valid selection when a preset is incompatible', () => {
    const previous = {
      preset_id: 'pulse' as const,
      config: { density: 'medium' as const, timespan_minutes: 15, alert_threshold: null },
    }
    expect(resolveBoardroomVisualizationSelection('human_realm', {
      preset_id: 'routes',
      config: { density: 'high', timespan_minutes: 60, alert_threshold: 0.5 },
    }, previous)).toEqual({
      ok: false,
      selection: previous,
      message: 'Routes is incompatible with human input; retained Pulse',
    })
  })
})
