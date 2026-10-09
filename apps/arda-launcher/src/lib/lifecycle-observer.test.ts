import { afterEach, describe, expect, it, vi } from 'vitest'
import { observeLifecycle } from './lifecycle-observer'
import type { LifecycleSnapshot } from './lifecycle'

const healthy = { aggregate_state: 'healthy' } as LifecycleSnapshot
const degraded = { aggregate_state: 'degraded' } as LifecycleSnapshot
const flush = () => vi.advanceTimersByTimeAsync(0)

afterEach(() => { vi.useRealTimers() })

describe('lifecycle observation lifetime', () => {
  it('does not start IPC when stopped before its first microtask', async () => {
    vi.useFakeTimers()
    const fetch = vi.fn().mockResolvedValue(healthy)
    const update = vi.fn()
    const observer = observeLifecycle(fetch, update)
    observer.stop()
    await observer.refresh()
    await flush()
    expect(fetch).not.toHaveBeenCalled()
    expect(update).not.toHaveBeenCalled()
    expect(vi.getTimerCount()).toBe(0)
  })

  it('refreshes required-service changes without operator action', async () => {
    vi.useFakeTimers()
    const fetch = vi.fn().mockResolvedValueOnce(healthy).mockResolvedValue(degraded)
    const update = vi.fn()
    const observer = observeLifecycle(fetch, update)
    await flush()
    expect(update).toHaveBeenLastCalledWith({ snapshot: healthy, error: null })
    await vi.advanceTimersByTimeAsync(2000)
    expect(update).toHaveBeenLastCalledWith({ snapshot: degraded, error: null })
    observer.stop()
  })

  it('invalidates healthy evidence on failure and recovers automatically', async () => {
    vi.useFakeTimers()
    const fetch = vi.fn().mockResolvedValueOnce(healthy).mockRejectedValueOnce(new Error('offline')).mockResolvedValue(degraded)
    const update = vi.fn()
    const observer = observeLifecycle(fetch, update)
    await flush()
    await vi.advanceTimersByTimeAsync(2000)
    expect(update).toHaveBeenLastCalledWith({ snapshot: null, error: expect.stringContaining('offline') })
    await vi.advanceTimersByTimeAsync(2000)
    expect(update).toHaveBeenLastCalledWith({ snapshot: degraded, error: null })
    observer.stop()
  })

  it('does not overlap requests, even when manually refreshed', async () => {
    vi.useFakeTimers()
    let resolve!: (s: LifecycleSnapshot) => void
    const fetch = vi.fn(() => new Promise<LifecycleSnapshot>(r => { resolve = r }))
    const observer = observeLifecycle(fetch, vi.fn())
    await flush()
    void observer.refresh()
    await vi.advanceTimersByTimeAsync(4000)
    expect(fetch).toHaveBeenCalledTimes(1)
    resolve(healthy)
    await flush()
    observer.stop()
  })

  it('expires a hung observation and refuses its late healthy result', async () => {
    vi.useFakeTimers()
    let resolve!: (s: LifecycleSnapshot) => void
    const fetch = vi.fn().mockResolvedValueOnce(healthy).mockImplementationOnce(() => new Promise<LifecycleSnapshot>(r => { resolve = r })).mockResolvedValue(degraded)
    const update = vi.fn()
    const observer = observeLifecycle(fetch, update)
    await flush()
    await vi.advanceTimersByTimeAsync(7000)
    expect(update).toHaveBeenLastCalledWith({ snapshot: null, error: expect.stringContaining('timed out') })
    resolve(healthy)
    await flush()
    expect(update).toHaveBeenLastCalledWith({ snapshot: null, error: expect.any(String) })
    await vi.advanceTimersByTimeAsync(2000)
    expect(update).toHaveBeenLastCalledWith({ snapshot: degraded, error: null })
    observer.stop()
  })

  it('ignores late completion after unmount and schedules no further work', async () => {
    vi.useFakeTimers()
    let resolve!: (s: LifecycleSnapshot) => void
    const fetch = vi.fn(() => new Promise<LifecycleSnapshot>(r => { resolve = r }))
    const update = vi.fn()
    const observer = observeLifecycle(fetch, update)
    await flush()
    observer.stop()
    resolve(healthy)
    await vi.advanceTimersByTimeAsync(10000)
    expect(update).not.toHaveBeenCalled()
    expect(fetch).toHaveBeenCalledTimes(1)
    expect(vi.getTimerCount()).toBe(0)
  })
})
