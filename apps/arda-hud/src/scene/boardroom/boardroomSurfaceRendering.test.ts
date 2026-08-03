import { describe, expect, it } from 'vitest'
import { resolveBoardroomSurfaceRenderStrategy } from './BoardroomViewport'

describe('boardroom native surface rendering', () => {
  it('renders visible monitor content through the native-safe HTML path', () => {
    expect(resolveBoardroomSurfaceRenderStrategy('monitor_surface', true)).toEqual({
      transform: false,
      rotation: undefined,
    })
  })

  it('renders visible desk content through the native-safe HTML path', () => {
    expect(resolveBoardroomSurfaceRenderStrategy('desk_surface', true)).toEqual({
      transform: false,
      rotation: undefined,
    })
  })

  it('preserves perspective transforms in browser rendering', () => {
    expect(resolveBoardroomSurfaceRenderStrategy('monitor_surface', false)).toEqual({
      transform: true,
      rotation: [0, 0, 0],
    })
    expect(resolveBoardroomSurfaceRenderStrategy('desk_surface', false)).toEqual({
      transform: true,
      rotation: [-Math.PI / 2, 0, 0],
    })
  })
})
