// sigil: REPAIR
export type BoardroomRenderContentKind =
  | 'empty'
  | 'component_grid'
  | 'service_status'
  | 'inline_embed'
  | 'remote_preview'
  | 'media_thumbnail'
  | 'stream_feed'
  | 'remote_desktop'

export interface BoardroomRenderContentContract {
  kind: BoardroomRenderContentKind
  enabled: boolean
  refreshMs: number
  detail: {
    sourceZoneId?: string
    slotId: string
    owner?: string
    payloadBinding?: string
    embedUrl?: string | null
    allowInlineEmbed?: boolean
    fallbackWidgets?: Array<{
      id: string
      kind: string
      title: string
      dataBinding: string
      gridArea: string
    }>
  }
}

export function createEmptyBoardroomRenderContent(slotId: string): BoardroomRenderContentContract {
  return {
    kind: 'empty',
    enabled: false,
    refreshMs: 5000,
    detail: { slotId },
  }
}
