// sigil: REPAIR
import { useCallback, useEffect, useRef, useState } from 'react'
import type { ArdaBundle, ArdaDataSource } from '../../../lib/ardaBundleTypes'

interface UseArdaBundleOptions {
  source: ArdaDataSource
  refreshIntervalMs?: number
  onLoaded?: (bundle: ArdaBundle) => void
}

export function useArdaBundle({ source, refreshIntervalMs = 5000, onLoaded }: UseArdaBundleOptions) {
  const [bundle, setBundle] = useState<ArdaBundle | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const sourceRef = useRef<ArdaDataSource>(source)
  sourceRef.current = source
  const initialLoadCompletedRef = useRef(false)
  const retryIntervalMsRef = useRef(refreshIntervalMs)
  const retryAttemptRef = useRef(0)
  const refreshTimerRef = useRef<number | null>(null)

  const cancelPendingRefresh = () => {
    if (refreshTimerRef.current) {
      window.clearTimeout(refreshTimerRef.current)
      refreshTimerRef.current = null
    }
  }

  const scheduleRetry = () => {
    cancelPendingRefresh()
    const nextDelay = Math.min(retryIntervalMsRef.current * Math.pow(2, retryAttemptRef.current), 30000)
    retryIntervalMsRef.current = nextDelay
    refreshTimerRef.current = window.setTimeout(() => {
      refreshTimerRef.current = null
      void sourceRef.current.loadBundle()
        .then((nextBundle) => applyBundle(nextBundle))
        .catch(() => applyBundle(null))
    }, nextDelay)
  }

  const applyBundle = useCallback((nextBundle: ArdaBundle | null) => {
    if (!nextBundle) {
      setBundle((current) => current)
      retryAttemptRef.current += 1
      setError((current) => current ?? 'Unable to load core-state bundle; retrying...')
      scheduleRetry()
      return
    }

    if (!initialLoadCompletedRef.current) {
      initialLoadCompletedRef.current = true
      onLoaded?.(nextBundle)
    }

    setBundle(nextBundle)
    setError(null)
    retryIntervalMsRef.current = refreshIntervalMs
    retryAttemptRef.current = 0
  }, [onLoaded, refreshIntervalMs])

  const load = useCallback(async () => {
    if (!initialLoadCompletedRef.current) {
      setIsLoading(true)
    }
    try {
      const nextBundle = await sourceRef.current.loadBundle()
      applyBundle(nextBundle)
    } catch (error) {
      console.error('[arda] bundle load failed', error)
      applyBundle(null)
    } finally {
      if (!initialLoadCompletedRef.current) {
        setIsLoading(false)
      }
    }
  }, [applyBundle])

  useEffect(() => {
    cancelPendingRefresh()
    initialLoadCompletedRef.current = false
    setIsLoading(true)
    retryAttemptRef.current = 0
    retryIntervalMsRef.current = refreshIntervalMs
    setError(null)
    setBundle(null)

    void load()

    const interval = window.setInterval(() => {
      cancelPendingRefresh()
      void load()
    }, refreshIntervalMs)

    return () => {
      clearInterval(interval)
      cancelPendingRefresh()
      setIsLoading(false)
      initialLoadCompletedRef.current = false
    }
  }, [load, refreshIntervalMs])

  return {
    bundle,
    error,
    isLoading,
    refreshBundle: load,
  }
}
