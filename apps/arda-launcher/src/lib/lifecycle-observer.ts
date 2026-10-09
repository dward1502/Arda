import type { LifecycleSnapshot } from './lifecycle'

export interface LifecycleObservation {
  snapshot: LifecycleSnapshot | null
  error: string | null
}

// Poll only after the prior request settles. A hung IPC cannot leave healthy
// evidence actionable, and a late result cannot reinstate expired evidence.
export function observeLifecycle(
  fetch: () => Promise<LifecycleSnapshot>,
  update: (observation: LifecycleObservation) => void,
) {
  let active = true
  let pending: Promise<void> | undefined
  let timer: ReturnType<typeof setTimeout> | undefined
  let deadline: ReturnType<typeof setTimeout> | undefined

  const refresh = (): Promise<void> => {
    if (!active) return Promise.resolve()
    if (pending) return pending
    clearTimeout(timer)
    let expired = false
    deadline = setTimeout(() => {
      expired = true
      if (active) update({ snapshot: null, error: 'Lifecycle status timed out; evidence unavailable.' })
    }, 5000)
    pending = Promise.resolve().then(() => active ? fetch() : null).then(snapshot => {
      if (active && !expired && snapshot) update({ snapshot, error: null })
    }).catch(cause => {
      if (active && !expired) update({ snapshot: null, error: `Lifecycle status unavailable: ${cause}` })
    }).finally(() => {
      clearTimeout(deadline)
      pending = undefined
      if (active) timer = setTimeout(() => { void refresh() }, 2000)
    })
    return pending
  }

  void refresh()
  return {
    refresh,
    stop() {
      active = false
      clearTimeout(timer)
      clearTimeout(deadline)
    },
  }
}
