// sigil: REPAIR
import MeterBar from './MeterBar'
import { fallbackProtocolMarker } from '../../lib/soterionRender'

type JWTier = 'optimal' | 'efficient' | 'nominal' | 'degraded' | 'critical'

interface JouleWorkCardProps {
  score: number
  energyIn?: number
  workOutput?: number
  wasteFactor?: number
  className?: string
}

function getTier(score: number): { tier: JWTier; label: string; color: string } {
  if (score >= 0.90) return { tier: 'optimal', label: 'OPTIMAL TIER', color: '#f9ca24' }
  if (score >= 0.75) return { tier: 'efficient', label: 'EFFICIENT TIER', color: '#ff6b35' }
  if (score >= 0.55) return { tier: 'nominal', label: 'NOMINAL TIER', color: '#e056fd' }
  if (score >= 0.35) return { tier: 'degraded', label: 'DEGRADED TIER', color: '#9b59b6' }
  return { tier: 'critical', label: 'CRITICAL TIER', color: '#ff3333' }
}

function getScoreGradient(tier: JWTier): string {
  switch (tier) {
    case 'optimal':
      return 'linear-gradient(135deg, #f9ca24, #ff6b35)'
    case 'efficient':
      return 'linear-gradient(135deg, #ff6b35, #e056fd)'
    case 'nominal':
      return 'linear-gradient(135deg, #e056fd, #9b59b6)'
    case 'degraded':
      return 'linear-gradient(135deg, #9b59b6, #ff3333)'
    case 'critical':
      return 'linear-gradient(135deg, #ff3333, #9b59b6)'
  }
}

export default function JouleWorkCard({
  score,
  energyIn,
  workOutput,
  wasteFactor,
  className = '',
}: JouleWorkCardProps) {
  const { tier, label, color } = getTier(score)
  const gradient = getScoreGradient(tier)
  const reviewMarker = fallbackProtocolMarker('REVIEW')

  return (
    <div
      className={`relative flex flex-col items-center gap-4 overflow-hidden border border-[#9b59b64d] bg-[#0a0014dd] p-6 backdrop-blur-xl ${className}`}
    >
      <div className="pointer-events-none absolute left-0 top-0 h-5 w-5 border-l-2 border-t-2 border-[#ff6b35]" />
      <div className="pointer-events-none absolute bottom-0 right-0 h-5 w-5 border-b-2 border-r-2 border-[#ff6b35]" />

      <div className="text-center">
        <div
          className="font-['Orbitron'] text-5xl font-black leading-none"
          style={{
            background: gradient,
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent',
            backgroundClip: 'text',
            filter: 'drop-shadow(0 0 20px rgba(255,107,53,0.5))',
          }}
        >
          {score.toFixed(2)}
        </div>
        <div className="mt-1 font-['Orbitron'] text-[8px] uppercase tracking-[4px] text-[#9b59b6]">
          Joulework Efficiency
        </div>
        <div
          className="mt-1 font-['Orbitron'] text-[10px] uppercase tracking-[3px]"
          style={{ color }}
        >
          {reviewMarker} {label}
        </div>
      </div>

      {(energyIn !== undefined || workOutput !== undefined || wasteFactor !== undefined) && (
        <div className="flex w-full max-w-[280px] flex-col gap-2">
          {energyIn !== undefined && (
            <MeterBar label="Energy In" value={energyIn} color="#ff6b35" />
          )}
          {workOutput !== undefined && (
            <MeterBar label="Work Output" value={workOutput} color="#f9ca24" />
          )}
          {wasteFactor !== undefined && (
            <MeterBar label="Waste Factor" value={wasteFactor} color="#ff3333" />
          )}
        </div>
      )}
    </div>
  )
}
