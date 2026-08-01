// sigil: REPAIR
import type { BoardroomConsoleShellSegment } from './boardroomSpatialLayout'

export interface BoardroomConsoleTrimBar {
  id: string
  args: [number, number, number]
  position: [number, number, number]
}

export interface BoardroomConsoleSegmentGeometry {
  housing: {
    position: [number, number, number]
    size: [number, number, number]
  }
  bezel: {
    position: [number, number, number]
    size: [number, number, number]
  }
  well: {
    position: [number, number, number]
    size: [number, number, number]
    emissiveIntensity: number
  }
  frame: BoardroomConsoleTrimBar[]
  vents: Array<{
    side: 'left' | 'right'
    position: [number, number, number]
    size: [number, number, number]
  }>
  fasteners: Array<{
    side: 'left' | 'right'
    index: number
    position: [number, number, number]
    size: [number, number, number]
  }>
}

function trimBar(
  id: string,
  args: [number, number, number],
  position: [number, number, number],
): BoardroomConsoleTrimBar {
  return { id, args, position }
}

export function deriveBoardroomConsoleSegmentGeometry(
  segment: BoardroomConsoleShellSegment,
): BoardroomConsoleSegmentGeometry {
  const [width, height, depth] = segment.size
  const bezelInset = 0.12
  const bezelThickness = 0.28
  const wellInset = 0.18
  const wellThickness = 0.035
  const bezelWidth = width - bezelInset * 2
  const bezelDepth = depth - bezelInset * 2
  const wellWidth = width - wellInset * 2
  const wellDepth = depth - wellInset * 2
  const bezelTop = height / 2 + bezelThickness
  const frameThickness = 0.045
  const frameHeight = 0.04
  const frameY = bezelTop + wellThickness + frameHeight / 2

  const bezel = {
    position: [0, height / 2 + bezelThickness / 2, 0] as [number, number, number],
    size: [bezelWidth, bezelThickness, bezelDepth] as [number, number, number],
  }

  const well = {
    position: [0, bezelTop + wellThickness / 2 + 0.004, 0] as [number, number, number],
    size: [wellWidth, wellThickness, wellDepth] as [number, number, number],
    emissiveIntensity: 0,
  }

  const frame = [
    trimBar('frame-back', [wellWidth + frameThickness * 2, frameHeight, frameThickness], [
      0,
      frameY,
      -wellDepth / 2 - frameThickness / 2,
    ]),
    trimBar('frame-front', [wellWidth + frameThickness * 2, frameHeight, frameThickness], [
      0,
      frameY,
      wellDepth / 2 + frameThickness / 2,
    ]),
    trimBar('frame-left', [frameThickness, frameHeight, wellDepth], [
      -wellWidth / 2 - frameThickness / 2,
      frameY,
      0,
    ]),
    trimBar('frame-right', [frameThickness, frameHeight, wellDepth], [
      wellWidth / 2 + frameThickness / 2,
      frameY,
      0,
    ]),
  ]

  const ventWidth = 0.55
  const ventHeight = 0.09
  const ventDepth = 0.055
  const fastenerSize: [number, number, number] = [0.07, 0.07, 0.04]
  const fastenerZ = depth / 2 + 0.025
  const fasteners = [-1, 1].flatMap((side) =>
    Array.from({ length: 2 }, (_, index) => {
      const x = side * (width / 2 - 0.18)
      const y = height / 2 - 0.3 - index * 0.28
      return {
        side: side === 1 ? ('right' as const) : ('left' as const),
        index,
        position: [x, y, fastenerZ] as [number, number, number],
        size: fastenerSize,
      }
    }),
  )

  return {
    housing: {
      position: segment.position,
      size: segment.size,
    },
    bezel,
    well,
    frame,
    vents: [
      {
        side: 'left',
        position: [-width / 2 + 0.32, 0.02, depth / 2 + 0.02],
        size: [ventWidth, ventHeight, ventDepth],
      },
      {
        side: 'right',
        position: [width / 2 - 0.32, 0.02, depth / 2 + 0.02],
        size: [ventWidth, ventHeight, ventDepth],
      },
    ],
    fasteners,
  }
}

export function validateBoardroomConsoleSegmentGeometry(geometry: BoardroomConsoleSegmentGeometry): string[] {
  const errors: string[] = []
  if (!geometry.housing.size.every((axis) => axis > 0)) {
    errors.push(`${geometry.housing.position.join(',')}: housing size must be positive`)
  }
  if (!geometry.bezel.size.every((axis) => axis > 0)) {
    errors.push(`${geometry.housing.position.join(',')}: bezel size must be positive`)
  }
  if (!geometry.well.size.every((axis) => axis > 0)) {
    errors.push(`${geometry.housing.position.join(',')}: well size must be positive`)
  }
  const bezelTop = geometry.bezel.position[1] + geometry.bezel.size[1] / 2
  const wellBottom = geometry.well.position[1] - geometry.well.size[1] / 2
  if (wellBottom < bezelTop) {
    errors.push(`${geometry.housing.position.join(',')}: screen well must remain above its housing`)
  }
  if (geometry.frame.length !== 4 || geometry.frame.some((bar) => !bar.args.every((axis) => axis > 0))) {
    errors.push(`${geometry.housing.position.join(',')}: screen well needs four positive frame rails`)
  }
  const leftVents = geometry.vents.filter((vent) => vent.side === 'left')
  const rightVents = geometry.vents.filter((vent) => vent.side === 'right')
  if (leftVents.length !== 1 || rightVents.length !== 1) {
    errors.push(`${geometry.housing.position.join(',')}: console segments need one vent per side`)
  }
  if (geometry.fasteners.length !== 4) {
    errors.push(`${geometry.housing.position.join(',')}: console segments need four fasteners`)
  }
  return errors
}
