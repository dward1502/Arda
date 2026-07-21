// sigil: REPAIR
interface MeterBarProps {
  label: string
  value: number
  color?: string
  className?: string
}

export default function MeterBar({
  label,
  value,
  color = '#ff6b35',
  className = '',
}: MeterBarProps) {
  const clamped = Math.max(0, Math.min(100, value))

  return (
    <div className={className}>
      <div className="mb-1 flex items-center justify-between font-['Share_Tech_Mono'] text-[10px] uppercase tracking-[0.12em] text-[#e8e8f0aa]">
        <span>{label}</span>
        <span>{Math.round(clamped)}%</span>
      </div>
      <div className="h-1.5 overflow-hidden rounded bg-[#ffffff10]">
        <div
          className="h-full transition-all duration-500"
          style={{
            width: `${clamped}%`,
            background: `linear-gradient(90deg, ${color}, #e056fd)`,
          }}
        />
      </div>
    </div>
  )
}
