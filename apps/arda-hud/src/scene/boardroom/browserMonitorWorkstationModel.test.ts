import { describe, expect, it } from 'vitest'
import {
  mapWorkstationPointerToCapture,
  normalizeBrowserAddress,
} from './browserMonitorWorkstationModel'

describe('browser monitor workstation model', () => {
  it('normalizes user-entered browser addresses without hard-coded destinations', () => {
    expect(normalizeBrowserAddress('youtube.com/watch?v=abc')).toBe('https://youtube.com/watch?v=abc')
    expect(normalizeBrowserAddress(' https://example.com/path ')).toBe('https://example.com/path')
    expect(() => normalizeBrowserAddress('file:///tmp/demo.mp4')).toThrow('HTTP(S)')
  })

  it('maps the visible contained browser frame into the 1280x720 capture viewport', () => {
    expect(mapWorkstationPointerToCapture({
      clientX: 640,
      clientY: 360,
      bounds: { left: 0, top: 0, width: 1280, height: 720 },
    })).toEqual({ x: 640, y: 360 })

    expect(mapWorkstationPointerToCapture({
      clientX: 640,
      clientY: 410,
      bounds: { left: 0, top: 50, width: 1280, height: 720 },
    })).toEqual({ x: 640, y: 360 })
  })

  it('rejects clicks in letterbox bars instead of sending wrong browser coordinates', () => {
    expect(mapWorkstationPointerToCapture({
      clientX: 50,
      clientY: 250,
      bounds: { left: 0, top: 0, width: 1280, height: 500 },
    })).toBeNull()
  })
})
