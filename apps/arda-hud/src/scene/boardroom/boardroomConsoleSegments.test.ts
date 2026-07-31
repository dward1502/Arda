// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  deriveBoardroomConsoleSegmentGeometry,
  validateBoardroomConsoleSegmentGeometry,
  type BoardroomConsoleSegmentGeometry,
} from './boardroomConsoleSegments'
import {
  BOARDROOM_CONSOLE_SHELL_SEGMENTS,
  type BoardroomConsoleShellSegment,
} from './boardroomSpatialLayout'

describe('boardroom console segment geometry', () => {
  const segmentGeometryById = BOARDROOM_CONSOLE_SHELL_SEGMENTS.reduce<Record<string, BoardroomConsoleSegmentGeometry>>(
    (accumulator, segment) => {
      accumulator[segment.id] = deriveBoardroomConsoleSegmentGeometry(segment)
      return accumulator
    },
    {},
  )

  it('derives housing, bezel, recessed well, frame, vent, and fastener geometry for every segment', () => {
    for (const segment of BOARDROOM_CONSOLE_SHELL_SEGMENTS) {
      const geometry = segmentGeometryById[segment.id]
      expect(geometry.housing.size).toEqual(segment.size)
      expect(geometry.bezel.size.every((axis) => axis > 0)).toBe(true)
      expect(geometry.well.size.every((axis) => axis > 0)).toBe(true)
      expect(geometry.frame).toHaveLength(4)
      expect(geometry.frame.map((bar) => bar.id)).toEqual([
        'frame-back',
        'frame-front',
        'frame-left',
        'frame-right',
      ])
      expect(geometry.frame.every((bar) => bar.args.every((axis) => axis > 0))).toBe(true)
      expect(geometry.vents).toHaveLength(2)
      expect(geometry.fasteners).toHaveLength(4)
      expect(geometry.bezel.position).toHaveLength(3)
      expect(geometry.well.position).toHaveLength(3)
      expect(geometry.vents.every((vent) => vent.position.length === 3)).toBe(true)
      expect(geometry.fasteners.every((fastener) => fastener.position.length === 3)).toBe(true)
      expect(geometry.vents).toEqual([
        expect.objectContaining({ side: 'left' }),
        expect.objectContaining({ side: 'right' }),
      ])
    }
  })

  it('preserves mirrored symmetry between left and right wings', () => {
    const outerLeft = segmentGeometryById['boardroom.console.outer_left']
    const outerRight = segmentGeometryById['boardroom.console.outer_right']
    const innerLeft = segmentGeometryById['boardroom.console.inner_left']
    const innerRight = segmentGeometryById['boardroom.console.inner_right']

    expect(outerLeft.housing.position[0]).toBeCloseTo(-outerRight.housing.position[0], 3)
    expect(innerLeft.housing.position[0]).toBeCloseTo(-innerRight.housing.position[0], 3)
    expect(outerLeft.bezel.position[0]).toBeCloseTo(-outerRight.bezel.position[0], 3)
    expect(innerLeft.bezel.position[0]).toBeCloseTo(-innerRight.bezel.position[0], 3)
    expect(outerLeft.vents[0].position[0]).toBeCloseTo(-outerRight.vents[1].position[0], 3)
    expect(innerLeft.fasteners[0].position[0]).toBeCloseTo(-innerRight.fasteners[3].position[0], 3)
  })

  it('keeps the screen well inside and below its raised frame', () => {
    for (const segment of BOARDROOM_CONSOLE_SHELL_SEGMENTS) {
      const geometry = segmentGeometryById[segment.id]
      const bezelTop = geometry.bezel.position[1] + geometry.bezel.size[1] / 2
      const wellBottom = geometry.well.position[1] - geometry.well.size[1] / 2
      const frameBottom = geometry.frame[0].position[1] - geometry.frame[0].args[1] / 2
      expect(wellBottom).toBeGreaterThanOrEqual(bezelTop)
      expect(frameBottom).toBeGreaterThan(geometry.well.position[1])
      expect(geometry.well.size[0]).toBeLessThan(geometry.bezel.size[0])
      expect(geometry.well.size[2]).toBeLessThan(geometry.bezel.size[2])
      expect(geometry.well.emissiveIntensity).toBe(0)
    }
  })

  it('validates console segment geometry and rejects impossible input', () => {
    const badSegment: BoardroomConsoleShellSegment = {
      id: 'boardroom.console.center',
      role: 'center' as const,
      position: [0, 0, 0],
      rotation: [0, 0, 0],
      size: [-1, 0.4, 2.08],
      accent: '#d8e7ff',
    }

    expect(validateBoardroomConsoleSegmentGeometry(deriveBoardroomConsoleSegmentGeometry(badSegment))).toEqual([
      '0,0,0: housing size must be positive',
      '0,0,0: bezel size must be positive',
      '0,0,0: well size must be positive',
      '0,0,0: screen well needs four positive frame rails',
    ])
  })
})
