import { describe, expect, it } from 'vitest'
import { resolveBoardroomInstrumentSurfaceGeometry } from './BoardroomInstrumentScreen'

describe('boardroom physical instrument surfaces', () => {
  it('fits upper content inside the authored monitor aperture', () => {
    const geometry = resolveBoardroomInstrumentSurfaceGeometry('boardroom.monitor.center', 'monitor_surface', [1.63, 0.8, 0.08])

    expect(geometry.width).toBeLessThan(1.63)
    expect(geometry.height).toBeLessThan(0.8)
    expect(geometry.position[2]).toBeGreaterThan(0.04)
    expect(geometry.rotation).toEqual([0, 0, 0])
  })

  it('lays lower content above the authored glass and bezel', () => {
    const geometry = resolveBoardroomInstrumentSurfaceGeometry('boardroom.control.center', 'desk_surface', [1.58, 0.04, 0.72])

    expect(geometry.width).toBeLessThan(1.58)
    expect(geometry.height).toBeLessThan(0.72)
    expect(geometry.position[1]).toBeGreaterThan(0.0355)
    expect(geometry.rotation).toEqual([-Math.PI / 2, 0, 0])
  })

  it('inherits the authored lower-desk surface-fit angles instead of cutting through the shell', () => {
    const ids = [
      'boardroom.lower.left_wrap',
      'boardroom.lower.left_inner',
      'boardroom.control.center',
      'boardroom.lower.right_inner',
      'boardroom.lower.right_wrap',
    ]
    const fits = ids.map((id) => resolveBoardroomInstrumentSurfaceGeometry(id, 'desk_surface', [1.58, 0.04, 0.72]))

    expect(fits.map((fit) => fit.fitPosition[1])).toEqual([0.07, 0.0275, 0, 0.0275, 0.07])
    expect(fits[0].fitRotation[2]).toBeGreaterThan(0)
    expect(fits[1].fitRotation[0]).toBeGreaterThan(0)
    expect(fits[2].fitRotation).toEqual([0, 0, 0])
    expect(fits[3].fitRotation[0]).toBeGreaterThan(0)
    expect(fits[4].fitRotation[2]).toBeLessThan(0)
  })
})
