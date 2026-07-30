import { useState, useEffect } from 'react'
import { Canvas } from '@react-three/fiber'
import ParticleSmoke from './components/ParticleSmoke'
import OnboardingText from './scenes/state/OnboardingText'
import WorldTree from './scenes/components/WorldTree'
import ArdaLogo from './components/ArdaLogo'
import OnboardingPanel from './components/OnboardingPanel'
import Background from './scenes/Background'
import {
  invokeOnboardingSnapshot,
  invokeRegistryStatus,
  type OnboardingSnapshot,
} from './lib/tauri-core-compat'

export type RegistryGate = 'loading' | 'pass' | 'warn' | 'fail'
export type OpenGate = 'locked' | 'open'

export interface GateState {
  phase: number
  registry: RegistryGate
  open: OpenGate
  statusLabel: string
  isReady: boolean
}

const INITIAL_STATE: GateState = {
  phase: 0,
  registry: 'loading',
  open: 'locked',
  statusLabel: 'Initializing...',
  isReady: false,
}

export default function App() {
  const [state, setState] = useState<GateState>(INITIAL_STATE)
  const [onboarding, setOnboarding] = useState<OnboardingSnapshot | null>(null)
  const [onboardingError, setOnboardingError] = useState<string | null>(null)
  const [onboardingLoading, setOnboardingLoading] = useState(false)
  const showLogo = state.phase >= 8.5

  useEffect(() => {
    let cancelled = false

    const tick = () => {
      setState((prev) => {
        if (cancelled) return prev
        const next = Math.min(prev.phase + 0.15, 11)
        return { ...prev, phase: next }
      })
    }

    const interval = setInterval(tick, 180)
    return () => {
      cancelled = true
      clearInterval(interval)
    }
  }, [])

  useEffect(() => {
    let cancelled = false
    loadRegistryGate()

    async function loadRegistryGate() {
      if (cancelled) return

      try {
        const result = await invokeRegistryStatus({})

        if (cancelled) return

        setState(prev => {
          if (result.loaded) {
            if (result.gate_status === 'pass') {
              return {
                ...prev,
                registry: 'pass',
                statusLabel: `Registry verified: ${result.track_count} tracks active`,
                isReady: true,
                open: prev.open,
              }
            }

            if (result.gate_status === 'warn') {
              return {
                ...prev,
                registry: 'warn',
                statusLabel: `Registry review: ${result.track_count} tracks loaded`,
                isReady: false,
              }
            }

            return {
              ...prev,
              registry: 'fail',
              statusLabel: 'Registry issues found',
              isReady: false,
            }
          }

          return {
            ...prev,
            registry: 'fail',
            statusLabel: result.error || 'Registry unavailable',
            isReady: false,
          }
        })
      } catch (e) {
        if (!cancelled) {
          setState(prev => ({
            ...prev,
            registry: 'fail',
            statusLabel: `Registry check failed: ${e}`,
            isReady: false,
          }))
        }
      }
    }

    return () => {
      cancelled = true
    }
  }, [])

  const onBegin = async () => {
    if (state.isReady === false) return

    if (state.open === 'open') {
      setState(prev => ({ ...prev, open: 'locked' }))
      return
    }

    setOnboardingLoading(true)
    setOnboardingError(null)
    setOnboarding(null)

    try {
      const snapshot = await invokeOnboardingSnapshot({})
      setOnboarding(snapshot)
    } catch (e) {
      console.error('Onboarding load failed', e)
      setOnboardingError(`Onboarding commands failed: ${e}`)
    } finally {
      setOnboardingLoading(false)
    }

    setState(prev => ({ ...prev, phase: 11, open: 'open' }))
  }

  const replay = () => {
    setState(INITIAL_STATE)
    setOnboarding(null)
    setOnboardingError(null)
    setOnboardingLoading(false)
  }

  return (
    <div className="w-screen h-screen bg-black overflow-hidden relative">
      <Canvas camera={{ position: [0, 2.5, 15], fov: 30 }}>
        <Background />

        {/* <Stars radius={280} depth={40} count={700} factor={2.8} fade /> */}
        <WorldTree phase={state.phase} />

        <ParticleSmoke />
      </Canvas>

      <OnboardingText
        phase={state.phase}
        registryStatus={state.registry}
        statusLabel={state.statusLabel}
        isReady={state.isReady}
      />
      <ArdaLogo show={showLogo} />
      {state.open === 'open' && (
        <OnboardingPanel
          snapshot={onboarding}
          error={onboardingError}
          onClose={() => setState(prev => ({ ...prev, open: 'locked' }))}
        />
      )}
      <div className="absolute bottom-1 left-1/2 -translate-x-1/2 flex gap-3 z-50">
        <button
          onClick={replay}
          className="px-4 py-1.5 text-xs font-mono border border-white/30 text-white/70 hover:bg-white/5 rounded"
        >
          REPLAY
        </button>
        <button
          onClick={onBegin}
          disabled={!state.isReady || onboardingLoading}
          className={`px-6 py-1.5 text-xs font-mono border rounded transition ${
            state.isReady
              ? 'border-[#f4e9d8]/60 text-[#f4e9d8] hover:bg-[#f4e9d8]/10'
              : 'border-white/20 text-white/40 cursor-not-allowed'
          }`}
        >
          {onboardingLoading ? 'LOADING' : state.open === 'open' ? 'CLOSE' : 'BEGIN'}
        </button>
        <div className="px-3 py-1.5 text-xs font-mono text-white/50 border border-white/20 rounded">
          PHASE {state.phase.toFixed(1)}
        </div>
      </div>
    </div>
  )
}
