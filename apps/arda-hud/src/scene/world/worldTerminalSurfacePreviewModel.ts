// sigil: REPAIR
import type { WorldSurfaceLayout } from '../../lib/worldSurfaceSettings'
import { deriveBoardroomSurfacePreviewModel, type BoardroomSurfacePreviewModel } from '../boardroom/boardroomSurfacePreviewModel'

export type WorldTerminalSurfaceId = 'terminal_queue' | 'terminal_tools' | 'terminal_status'

export interface WorldTerminalSurfacePreviewModel extends BoardroomSurfacePreviewModel {
  terminalId: string
  sourceZoneId: string
  adapterType: string
  focusMode: string
  safeActionSummary: string
  feedLines: string[]
}

const TERMINAL_ACTION_SUMMARIES: Record<WorldTerminalSurfaceId, string> = {
  terminal_queue: 'queue preview / task pivot draft',
  terminal_tools: 'setup audit / repair preview',
  terminal_status: 'runtime checks / provider refresh',
}

function terminalTitle(terminalId: string): string {
  if (terminalId === 'terminal_queue') return 'Queue Terminal'
  if (terminalId === 'terminal_tools') return 'Tools Terminal'
  if (terminalId === 'terminal_status') return 'Status Terminal'
  return terminalId.split('_').filter(Boolean).map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' ')
}

function actionSummary(terminalId: string): string {
  return TERMINAL_ACTION_SUMMARIES[terminalId as WorldTerminalSurfaceId] ?? 'focused source inspection'
}

export function deriveWorldTerminalSurfacePreviewModel({
  terminalId,
  layout,
}: {
  terminalId: string
  layout: WorldSurfaceLayout | undefined
}): WorldTerminalSurfacePreviewModel {
  const fallbackLayout: WorldSurfaceLayout = {
    enabled: false,
    adapter_type: 'streaming_text',
    preview: {
      mode: 'stream_feed',
      refresh_ms: 1500,
      widgets: [],
    },
    focus: {
      mode: 'in_scene_workstation',
      target: terminalId,
      refresh_ms: 1000,
    },
    embed: {
      url: null,
      allow_inline: false,
    },
  }
  const resolvedLayout = layout ?? fallbackLayout
  const baseModel = deriveBoardroomSurfacePreviewModel({
    title: terminalTitle(terminalId),
    layout: resolvedLayout,
  })

  return {
    ...baseModel,
    terminalId,
    sourceZoneId: resolvedLayout.focus.target,
    adapterType: resolvedLayout.adapter_type,
    focusMode: resolvedLayout.focus.mode,
    safeActionSummary: actionSummary(terminalId),
    feedLines: resolvedLayout.preview.widgets.slice(0, 3).map((widget) => `${widget.title} :: ${widget.data_binding}`),
  }
}
