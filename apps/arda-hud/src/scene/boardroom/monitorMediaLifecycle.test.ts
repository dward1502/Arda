import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  createMonitorMediaLifecycle,
  getMonitorMediaResourceSnapshot,
  resetMonitorMediaResourceSnapshotForTests,
} from './monitorMediaLifecycle'

describe('monitor media lifecycle', () => {
  beforeEach(() => resetMonitorMediaResourceSnapshotForTests())

  it('returns to zero active resources after repeated replacement cycles', () => {
    const cancelAnimationFrame = vi.fn()

    for (let cycle = 0; cycle < 25; cycle += 1) {
      const lifecycle = createMonitorMediaLifecycle(cancelAnimationFrame)
      const image = { onload: vi.fn(), onerror: vi.fn(), removeAttribute: vi.fn() }
      const video = {
        pause: vi.fn(),
        removeAttribute: vi.fn(),
        load: vi.fn(),
      }

      lifecycle.registerImage(image)
      lifecycle.registerVideo(video)
      lifecycle.scheduleAnimationFrame(cycle + 1)
      lifecycle.dispose()
      lifecycle.dispose()

      expect(image.onload).toBeNull()
      expect(image.onerror).toBeNull()
      expect(image.removeAttribute).toHaveBeenCalledWith('src')
      expect(video.pause).toHaveBeenCalledTimes(1)
      expect(video.removeAttribute).toHaveBeenCalledWith('src')
      expect(video.load).toHaveBeenCalledTimes(1)
    }

    expect(getMonitorMediaResourceSnapshot()).toEqual({
      activeImages: 0,
      activeVideos: 0,
      activeUnmutedVideos: 0,
      activeAnimationFrames: 0,
      completedDisposals: 25,
    })
    expect(cancelAnimationFrame).toHaveBeenCalledTimes(25)
  })
})
