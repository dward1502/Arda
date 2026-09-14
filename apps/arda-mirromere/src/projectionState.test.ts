import type { MirromereSurface } from '@arda/mirromere-ui'
import { describe, expect, it } from 'vitest'
import {
  isProjectionVeiled,
  selectableDisplays,
  type DisplayState,
} from './projectionState'

const displayState: DisplayState = {
  displays: [
    { id: 'primary', name: 'Primary', is_primary: true, position: [0, 0], size: [1920, 1080], scale_factor: 1 },
    { id: 'outpost', name: 'Outpost', is_primary: false, position: [1920, 0], size: [1920, 1080], scale_factor: 1 },
  ],
  selected_display_id: 'outpost',
  projected: true,
  veil_reason: null,
}
const surface = {} as MirromereSurface

describe('standalone projection state', () => {
  it('renders only when a surface is projected without a veil reason', () => {
    expect(isProjectionVeiled(displayState, surface)).toBe(false)
  })

  it.each([
    ['display state is unavailable', null, surface],
    ['surface is unavailable', displayState, null],
    ['projection is blocked', { ...displayState, projected: false }, surface],
    ['display recovery requires a veil', { ...displayState, veil_reason: 'selected_display_unavailable' }, surface],
  ])('fails closed when %s', (_name, state, projectedSurface) => {
    expect(isProjectionVeiled(state, projectedSurface)).toBe(true)
  })

  it('never offers the primary display as a projection target', () => {
    expect(selectableDisplays(displayState).map(display => display.id)).toEqual(['outpost'])
  })
})
