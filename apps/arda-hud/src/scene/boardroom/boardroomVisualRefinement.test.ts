// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { BOARDROOM_CONTROL_ZONES, BOARDROOM_MONITOR_ZONES } from './boardroomSpatialLayout'
import { deriveBoardroomScreenVisualRefinement, validateBoardroomVisualRefinement } from './boardroomVisualRefinement'

describe('boardroom visual refinement', () => {
  it('adds glass and trim requirements to upper monitor surfaces', () => {
    const refinement = deriveBoardroomScreenVisualRefinement(BOARDROOM_MONITOR_ZONES[0])

    expect(refinement.trimBars.map((bar) => bar.id)).toEqual(['top', 'bottom', 'left', 'right'])
    expect(refinement.paneOpacity).toBeLessThan(1)
    expect(refinement.paneClearcoat).toBeGreaterThan(0.5)
    expect(refinement.trimMetalness).toBeGreaterThanOrEqual(0.5)
    expect(refinement.paneArgs[2]).toBeLessThan(0.08)
  })

  it('adds horizontal trim to lower desk surfaces', () => {
    const lowerSurface = BOARDROOM_CONTROL_ZONES.find((zone) => zone.kind === 'desk_surface')!
    const refinement = deriveBoardroomScreenVisualRefinement(lowerSurface)

    expect(refinement.trimBars.map((bar) => bar.id)).toEqual(['front', 'back', 'left', 'right'])
    expect(refinement.paneArgs[1]).toBeLessThan(0.08)
    expect(refinement.trimBars[0].args[2]).toBeLessThan(0.08)
  })

  it('validates all boardroom screen slots for first-pass visual refinement', () => {
    const screenZones = [...BOARDROOM_MONITOR_ZONES, ...BOARDROOM_CONTROL_ZONES.filter((zone) => zone.kind === 'desk_surface')]

    expect(validateBoardroomVisualRefinement(screenZones)).toEqual([])
  })
})
