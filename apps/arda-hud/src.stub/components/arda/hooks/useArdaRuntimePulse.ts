// sigil: REPAIR
import { useEffect, useState } from 'react'
import type { LiveRuntimeChannelEvidence } from '../modules/OperatingSurfacePlanModule'
import {
  listenHudPulse,
  startHudPulseStream,
  stopHudPulseStream,
  type HudPulseEvent,
} from '../../../lib/weathertop'

export function useArdaRuntimePulse(intervalMs = 5000) {
  const [liveRuntime, setLiveRuntime] = useState<LiveRuntimeChannelEvidence | null>(null)

  useEffect(() => {
    let disposed = false
    let unlisten: (() => void) | null = null

    const attachHudPulse = async () => {
      try {
        unlisten = await listenHudPulse((event: HudPulseEvent) => {
          if (disposed) return
          setLiveRuntime({
            channel: 'arda://hud-pulse',
            source: event.source,
            status: event.status,
            sequence: event.sequence,
            lastEventIso: new Date(event.ts_unix_ms).toISOString(),
            durableProjection: 'ArdaBundle projection remains loaded from core/state projections',
          })
        })
        await startHudPulseStream(intervalMs)
      } catch (error) {
        console.warn('HUD pulse stream unavailable', error)
      }
    }

    void attachHudPulse()

    return () => {
      disposed = true
      if (unlisten) unlisten()
      void stopHudPulseStream().catch(() => undefined)
    }
  }, [intervalMs])

  return liveRuntime
}
