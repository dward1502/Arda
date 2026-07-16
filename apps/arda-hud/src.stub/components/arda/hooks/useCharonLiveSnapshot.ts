// sigil: REPAIR
import { useCallback, useEffect, useRef, useState } from 'react'
import { loadCharonLiveSnapshot, type CharonLiveSnapshot } from '../../../lib/charonLive'

export function useCharonLiveSnapshot(refreshIntervalMs = 5000) {
  const [snapshot, setSnapshot] = useState<CharonLiveSnapshot | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const pendingCount = useRef(0)

  const refresh = useCallback(async () => {
    try {
      pendingCount.current += 1
      const nextSnapshot = await loadCharonLiveSnapshot()
      pendingCount.current -= 1
      if (!pendingCount.current) {
        setSnapshot((current) => {
          return Object.is(current, nextSnapshot) ? current : nextSnapshot
        })
        setError(null)
      }
    } catch (loadError) {
      pendingCount.current -= 1
      if (!pendingCount.current) {
        setError(loadError instanceof Error ? loadError.message : 'Unknown Charon live snapshot failure')
      }
    } finally {
      if (!pendingCount.current) {
        setIsLoading(false)
      }
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    const load = async () => {
      if (!cancelled) {
        await refresh()
      }
    }
    void load()
    const interval = window.setInterval(() => void load(), refreshIntervalMs)
    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [refresh, refreshIntervalMs])

  return { snapshot, error, isLoading, refresh }
}
