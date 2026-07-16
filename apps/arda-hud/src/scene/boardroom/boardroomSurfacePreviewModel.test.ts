import { describe, expect, it } from 'vitest'
import type { BoardroomSurfaceLayout } from '../../lib/boardroomSlotSettings'
import { deriveBoardroomSurfacePreviewModel, type BoardroomSurfacePreviewWidgetModel } from './boardroomSurfacePreviewModel'

const baseLayout: BoardroomSurfaceLayout = {
  enabled: true,
  adapter_type: 'streaming_text',
  preview: {
    mode: 'stream_feed',
    refresh_ms: 1000,
    widgets: [
      { id: 'wm1', kind: 'data_stream', title: 'Raw source', data_binding: 'source.json', grid_area: 'main' },
      { id: 'wm2', kind: 'markdown_doc', title: 'Raw notes', data_binding: 'notes.md', grid_area: 'main' },
    ],
  },
  focus: { mode: 'native_window', target: 'raw_source', refresh_ms: 1000 },
  embed: { url: null, allow_inline: false },
}

describe('boardroom surface preview model', () => {
  it('derives generic preview metadata from surface widgets', () => {
    const model = deriveBoardroomSurfacePreviewModel({ title: 'System View', layout: baseLayout })
    expect(model.title).toBe('System View')
    expect(model.status).toBe('nominal')
    expect(model.widgets.map((widget) => widget.kind)).toEqual(['data_stream', 'markdown_doc'])
    expect(model.widgets.map((widget) => widget.mediaLabel)).toEqual(['DATA', 'MD'])
    expect(model.widgets[0].values).toHaveLength(4)
  })

  it('produces safe attention fallback when metadata is missing', () => {
    const model = deriveBoardroomSurfacePreviewModel({ title: 'Unknown Surface', layout: { ...baseLayout, enabled: false } })
    expect(model.status).toBe('disabled')
    expect(model.widgets[0].status).toBe('disabled')
  })

  it('enriches fleet/system roles with role-aware widgets', () => {
    const model = deriveBoardroomSurfacePreviewModel({
      title: 'Systems Health',
      layout: baseLayout,
      sourceZoneId: 'systems_health',
    })

    expect(model.status).not.toBe('raw_json')
    expect(model.widgets.some((widget: BoardroomSurfacePreviewWidgetModel) => ['status_grid', 'sparkline', 'agent_comms'].includes(widget.kind))).toBe(true)
  })

  it('falls back to generic widgets for unknown roles', () => {
    const model = deriveBoardroomSurfacePreviewModel({
      title: 'Service Warp',
      layout: baseLayout,
      sourceZoneId: 'service_warp_dev',
    })

    const kinds = model.widgets.map((widget: BoardroomSurfacePreviewWidgetModel) => widget.kind)
    expect(kinds).toEqual(['data_stream', 'markdown_doc'])
  })
})
