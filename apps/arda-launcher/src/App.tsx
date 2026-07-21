import { useState, useEffect } from 'react'
import { Canvas } from '@react-three/fiber'
import ParticleSmoke from './components/ParticleSmoke'
import OnboardingText from './scenes/state/OnboardingText'
import WorldTree from './scenes/components/WorldTree'
import ArdaLogo from './components/ArdaLogo'
import Background from './scenes/Background'

export default function App() {
  const [phase, setPhase] = useState(0)
  const showLogo = phase >= 8.5
  const replay = () => setPhase(0)

  useEffect(() => {
    const interval = setInterval(() => {
      setPhase(p => Math.min(p + 0.15, 11))
    }, 180)
    return () => clearInterval(interval)
  }, [])

  return (
    <div className="w-screen h-screen bg-black overflow-hidden relative">
      <Canvas camera={{ position: [0, 2.5, 15], fov: 30 }}>
        <Background />

        {/* <Stars radius={280} depth={40} count={700} factor={2.8} fade /> */}
        <WorldTree phase={phase} />

        <ParticleSmoke />
      </Canvas>

      <OnboardingText phase={phase} />
      <ArdaLogo show={showLogo} />
      <div className="absolute bottom-1 left-1/2 -translate-x-1/2 flex gap-3 z-50">
        <button
          onClick={replay}
          className="px-4 py-1.5 text-xs font-mono border border-white/30 text-white/70 hover:bg-white/5 rounded"
        >
          REPLAY
        </button>
        <div className="px-3 py-1.5 text-xs font-mono text-white/50 border border-white/20 rounded">
          PHASE {phase.toFixed(1)}
        </div>
      </div>
    </div>
  )
}
