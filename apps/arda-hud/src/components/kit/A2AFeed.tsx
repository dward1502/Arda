// sigil: REPAIR
import { useRef, useEffect } from 'react'

export interface A2AMessage {
  id: string
  from: string
  to: string
  text: string
  timestamp?: Date
}

interface A2AFeedProps {
  messages: A2AMessage[]
  agentColors?: Record<string, string>
  maxLines?: number
  className?: string
}

const defaultAgentColors: Record<string, string> = {
  ARANDUR: '#ff6b35',
  ATHENA: '#9b59b6',
  ORACLE: '#e056fd',
  ARES: '#ff6b35',
  PLUTUS: '#f9ca24',
  HADES: '#9b59b6',
  HERMES: '#9b59b6',
  NEMESIS: '#ff3333',
  MUSE: '#e056fd',
}

export default function A2AFeed({
  messages,
  agentColors = defaultAgentColors,
  maxLines = 20,
  className = '',
}: A2AFeedProps) {
  const feedRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (feedRef.current) {
      feedRef.current.scrollTop = feedRef.current.scrollHeight
    }
  }, [messages.length])

  const visible = messages.slice(-maxLines)

  return (
    <div
      ref={feedRef}
      className={`max-h-32 overflow-y-auto font-['Share_Tech_Mono'] text-[10px] leading-[1.8] text-[#e8e8f066] ${className}`}
    >
      {visible.map((msg) => (
        <div key={msg.id} className="flex gap-1">
          <span
            className="shrink-0 font-semibold"
            style={{ color: agentColors[msg.from] ?? '#9b59b6' }}
          >
            {msg.from}
          </span>
          <span className="text-[#9b59b644]">→</span>
          <span
            className="shrink-0"
            style={{ color: agentColors[msg.to] ?? '#9b59b6' }}
          >
            {msg.to}
          </span>
          <span className="text-[#9b59b644]">:</span>
          <span className="truncate text-[#e8e8f099]">{msg.text}</span>
        </div>
      ))}

      {visible.length === 0 && (
        <div className="py-2 text-center text-[#9b59b644]">
          No agent traffic
        </div>
      )}
    </div>
  )
}
