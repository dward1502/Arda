// sigil: REPAIR
import type { BoardroomSpatialZone } from './boardroomSpatialLayout'

export interface BoardroomScreenTrimBar {
  id: string
  args: [number, number, number]
  position: [number, number, number]
}

export interface BoardroomScreenVisualRefinement {
  paneArgs: [number, number, number]
  paneOpacity: number
  paneRoughness: number
  paneMetalness: number
  paneClearcoat: number
  trimBars: BoardroomScreenTrimBar[]
  trimRoughness: number
  trimMetalness: number
  trimEmissiveIntensity: number
}

function trimBar(id: string, args: [number, number, number], position: [number, number, number]): BoardroomScreenTrimBar {
  return { id, args, position }
}

export function deriveBoardroomScreenVisualRefinement(zone: BoardroomSpatialZone): BoardroomScreenVisualRefinement {
  const isUpperMonitor = zone.kind === 'upper_monitor'
  const paneArgs: [number, number, number] = isUpperMonitor
    ? [zone.size[0] * 0.92, zone.size[1] * 0.88, 0.055]
    : [zone.size[0] * 0.94, 0.055, zone.size[2] * 0.86]
  const [width, height, depth] = paneArgs
  const edge = isUpperMonitor ? 0.045 : 0.052
  const trimDepth = isUpperMonitor ? depth * 1.8 : edge
  const trimHeight = isUpperMonitor ? edge : height * 1.65
  const trimY = isUpperMonitor ? height / 2 + edge * 0.52 : 0
  const trimZ = isUpperMonitor ? 0 : depth / 2 + edge * 0.52

  const trimBars = isUpperMonitor
    ? [
      trimBar('top', [width + edge * 2.2, edge, trimDepth], [0, trimY, 0]),
      trimBar('bottom', [width + edge * 2.2, edge, trimDepth], [0, -trimY, 0]),
      trimBar('left', [edge, height, trimDepth], [-(width / 2 + edge * 0.55), 0, 0]),
      trimBar('right', [edge, height, trimDepth], [width / 2 + edge * 0.55, 0, 0]),
    ]
    : [
      trimBar('front', [width + edge * 2.2, trimHeight, edge], [0, 0, trimZ]),
      trimBar('back', [width + edge * 2.2, trimHeight, edge], [0, 0, -trimZ]),
      trimBar('left', [edge, trimHeight, depth], [-(width / 2 + edge * 0.55), 0, 0]),
      trimBar('right', [edge, trimHeight, depth], [width / 2 + edge * 0.55, 0, 0]),
    ]

  return {
    paneArgs,
    paneOpacity: isUpperMonitor ? 0.72 : 0.62,
    paneRoughness: isUpperMonitor ? 0.13 : 0.18,
    paneMetalness: 0.16,
    paneClearcoat: isUpperMonitor ? 0.72 : 0.58,
    trimBars,
    trimRoughness: 0.36,
    trimMetalness: 0.72,
    trimEmissiveIntensity: isUpperMonitor ? 0.52 : 0.4,
  }
}

export function validateBoardroomVisualRefinement(zones: BoardroomSpatialZone[]): string[] {
  return zones.flatMap((zone) => {
    const refinement = deriveBoardroomScreenVisualRefinement(zone)
    const errors: string[] = []
    if (refinement.trimBars.length < 4) errors.push(`${zone.id}: missing four trim bars`)
    if (refinement.paneClearcoat <= 0) errors.push(`${zone.id}: glass clearcoat not configured`)
    if (refinement.paneOpacity >= 1) errors.push(`${zone.id}: glass pane must remain translucent`)
    if (refinement.trimMetalness < 0.5) errors.push(`${zone.id}: trim metalness below cockpit target`)
    return errors
  })
}
