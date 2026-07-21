// sigil: REPAIR
interface SectionHeaderProps {
  title: string
  subtitle?: string
}

export default function SectionHeader({ title, subtitle }: SectionHeaderProps) {
  return (
    <div>
      <div className="mb-1 flex items-center gap-2 font-['Orbitron'] text-[10px] uppercase tracking-[0.28em] text-[#ff6b35]">
        <span className="inline-block h-px w-6 bg-[#ff6b35]" />
        {title}
        <span className="inline-block h-px flex-1 bg-gradient-to-r from-[#ff6b3566] to-transparent" />
      </div>
      {subtitle ? (
        <p className="pl-8 font-['Share_Tech_Mono'] text-[10px] tracking-[0.12em] text-[#e8e8f066]">
          {subtitle}
        </p>
      ) : null}
    </div>
  )
}
