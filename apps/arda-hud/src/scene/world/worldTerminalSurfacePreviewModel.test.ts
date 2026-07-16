// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import type { WorldSurfaceLayout } from '../../lib/worldSurfaceSettings'
import { deriveWorldTerminalSurfacePreviewModel } from './worldTerminalSurfacePreviewModel'

const terminalLayout: WorldSurfaceLayout = {
  enabled: true,
  adapter_type: 'streaming_text',
  preview: {
    mode: 'stream_feed',
    refresh_ms: 1500,
    widgets: [
      { id: 'terminal_queue.status', kind: 'status_grid', title: 'Terminal Status', data_binding: 'planning_and_queue.status', grid_area: 'top' },
      { id: 'terminal_queue.feed', kind: 'agent_comms', title: 'Activity Feed', data_binding: 'planning_and_queue.feed', grid_area: 'main' },
    ],
  },
  focus: {
    mode: 'in_scene_workstation',
    target: 'planning_and_queue',
    refresh_ms: 1000,
  },
  embed: {
    url: null,
    allow_inline: false,
  },
}

describe('world terminal surface preview model', () => {
  it('derives terminal-specific preview metadata from surface layout widgets', () => {
    const model = deriveWorldTerminalSurfacePreviewModel({
      terminalId: 'terminal_queue',
      layout: terminalLayout,
    })

    expect(model.title).toBe('Queue Terminal')
    expect(model.sourceZoneId).toBe('planning_and_queue')
    expect(model.adapterType).toBe('streaming_text')
    expect(model.focusMode).toBe('in_scene_workstation')
    expect(model.safeActionSummary).toBe('queue preview / task pivot draft')
    expect(model.widgets.map((widget) => widget.kind)).toEqual(['status_grid', 'agent_comms'])
    expect(model.feedLines).toEqual([
      'Terminal Status :: planning_and_queue.status',
      'Activity Feed :: planning_and_queue.feed',
    ])
  })

  it('returns a disabled fallback for unconfigured terminal layouts', () => {
    const model = deriveWorldTerminalSurfacePreviewModel({
      terminalId: 'terminal_future',
      layout: undefined,
    })

    expect(model.title).toBe('Terminal Future')
    expect(model.status).toBe('disabled')
    expect(model.sourceZoneId).toBe('terminal_future')
    expect(model.widgets).toEqual([])
    expect(model.safeActionSummary).toBe('focused source inspection')
  })
})
