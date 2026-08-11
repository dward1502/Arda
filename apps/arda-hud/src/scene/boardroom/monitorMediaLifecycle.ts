export interface MonitorMediaResourceSnapshot {
  activeImages: number
  activeVideos: number
  activeUnmutedVideos: number
  activeAnimationFrames: number
  completedDisposals: number
}

interface DisposableImage {
  onload: unknown
  onerror: unknown
  removeAttribute?(name: string): void
}

interface DisposableVideo {
  muted?: boolean
  pause(): void
  removeAttribute(name: string): void
  load(): void
}

const resources: MonitorMediaResourceSnapshot = {
  activeImages: 0,
  activeVideos: 0,
  activeUnmutedVideos: 0,
  activeAnimationFrames: 0,
  completedDisposals: 0,
}

export function getMonitorMediaResourceSnapshot(): MonitorMediaResourceSnapshot {
  return { ...resources }
}

export function resetMonitorMediaResourceSnapshotForTests(): void {
  resources.activeImages = 0
  resources.activeVideos = 0
  resources.activeUnmutedVideos = 0
  resources.activeAnimationFrames = 0
  resources.completedDisposals = 0
}

export function createMonitorMediaLifecycle(cancelFrame: (frameId: number) => void) {
  let image: DisposableImage | null = null
  let video: DisposableVideo | null = null
  let videoWasUnmuted = false
  let animationFrame = 0
  let disposed = false

  return {
    registerImage(next: DisposableImage): void {
      if (disposed || image === next) return
      if (image) {
        image.onload = null
        image.onerror = null
        resources.activeImages -= 1
      }
      image = next
      resources.activeImages += 1
    },
    registerVideo(next: DisposableVideo): void {
      if (disposed || video === next) return
      if (video) {
        video.pause()
        video.removeAttribute('src')
        video.load()
        resources.activeVideos -= 1
        if (videoWasUnmuted) resources.activeUnmutedVideos -= 1
      }
      video = next
      videoWasUnmuted = next.muted !== true
      resources.activeVideos += 1
      if (videoWasUnmuted) resources.activeUnmutedVideos += 1
    },
    scheduleAnimationFrame(frameId: number): void {
      if (disposed || frameId <= 0) return
      if (!animationFrame) resources.activeAnimationFrames += 1
      animationFrame = frameId
    },
    dispose(): void {
      if (disposed) return
      disposed = true
      if (animationFrame) {
        cancelFrame(animationFrame)
        animationFrame = 0
        resources.activeAnimationFrames -= 1
      }
      if (image) {
        image.onload = null
        image.onerror = null
        image.removeAttribute?.('src')
        image = null
        resources.activeImages -= 1
      }
      if (video) {
        video.pause()
        video.removeAttribute('src')
        video.load()
        video = null
        resources.activeVideos -= 1
        if (videoWasUnmuted) resources.activeUnmutedVideos -= 1
      }
      resources.completedDisposals += 1
    },
  }
}
