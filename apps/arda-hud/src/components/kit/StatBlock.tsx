// sigil: REPAIR
type DeltaType = 'positive' | 'negative' | 'neutral'

interface StatBlockProps {
  label: string
  value: string | number
  delta?: string
  deltaType?: DeltaType
  className?: string
}

const deltaStyles: Record<DeltaType, string> = {
  positive: 'text-[#f9ca24]',
  negative: 'text-[#ff3333]',
  neutral: 'text-[#9b59b6]',
}

export default function StatBlock({
  label,
  value,
  delta,
  deltaType = 'neutral',
  className = '',
}: StatBlockProps) {
  return (
    <div
      className={`flex min-w-[120px] flex-col gap-1 border border-[rgba(155,89,182,0.2)] border-b-2 border-b-[#ff6b35] bg-[rgba(20,0,32,0.7)] p-3 backdrop-blur-sm ${className}`}
    >
      <div className="font-['Orbitron'] text-[7px] uppercase tracking-[0.18em] text-[#9b59b6]">
        {label}
      </div>
      <div className="font-['Share_Tech_Mono'] text-[28px] leading-none text-[#ff6b35]">
        {value}
      </div>
      {delta && (
        <div className={`font-['Share_Tech_Mono'] text-[10px] ${deltaStyles[deltaType]}`}>
          {delta}
        </div>
      )}
    </div>
  )
}
