// sigil: REPAIR
import { useRef, useEffect, type KeyboardEvent } from 'react'

export type FeedLineType = 'user' | 'response' | 'system' | 'alert' | 'success'

export interface FeedLine {
  id: string
  type: FeedLineType
  agent?: string
  agentColor?: string
  text: string
  timestamp?: Date
}

interface CommandFeedProps {
  lines: FeedLine[]
  placeholder?: string
  promptSymbol?: string
  promptColor?: string
  onSubmit?: (value: string) => void
  maxHeight?: number
  className?: string
  showInput?: boolean
}

const lineColors: Record<FeedLineType, string> = {
  user: 'text-[#e8e8f0]',
  response: 'text-[#e8e8f0aa]',
  system: 'text-[#9b59b6] text-[11px]',
  alert: 'text-[#ff3333]',
  success: 'text-[#f9ca24]',
}

export default function CommandFeed({
  lines,
  placeholder = 'Speak to Arandur...',
  promptSymbol = '›',
  promptColor = '#ff6b35',
  onSubmit,
  maxHeight = 240,
  className = '',
  showInput = true,
}: CommandFeedProps) {
  const outputRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight
    }
  }, [lines.length])

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && inputRef.current) {
      const val = inputRef.current.value.trim()
      if (val && onSubmit) {
        onSubmit(val)
        inputRef.current.value = ''
      }
    }
  }

  return (
    <div
      className={`overflow-hidden border border-[#9b59b633] border-l-2 border-l-[#ff6b35] bg-[#0a0010cc] shadow-[inset_0_0_40px_rgba(0,0,0,0.3)] backdrop-blur-md ${className}`}
    >

      <div
        ref={outputRef}
        className="overflow-y-auto px-4 py-3 font-['Share_Tech_Mono'] text-xs leading-[1.8] scrollbar-thin scrollbar-track-[#0a0010] scrollbar-thumb-[#ff6b3566]"
        style={{ maxHeight }}
      >
        {lines.map((line) => (
          <div key={line.id} className={`flex gap-2 py-0.5 ${lineColors[line.type]}`}>
            {line.type === 'system' ? (
              <span>// {line.text}</span>
            ) : (
              <>
                {line.agent && (
                  <span
                    className="shrink-0 whitespace-nowrap"
                    style={{ color: line.agentColor ?? '#ff6b35' }}
                  >
                    {line.agent} ›
                  </span>
                )}
                <span>{line.text}</span>
              </>
            )}
          </div>
        ))}

        {lines.length === 0 && (
          <div className="py-4 text-center font-['Share_Tech_Mono'] text-[11px] text-[#9b59b6]">
            No messages yet
          </div>
        )}
      </div>


      {showInput && (
        <div className="flex items-center gap-2 border-t border-[#9b59b666] px-4 py-2">
          <span
            className="shrink-0 font-['Share_Tech_Mono'] text-sm"
            style={{ color: promptColor }}
          >
            {promptSymbol}
          </span>
          <input
            ref={inputRef}
            type="text"
            placeholder={placeholder}
            onKeyDown={handleKeyDown}
            className="flex-1 border-none bg-transparent font-['Share_Tech_Mono'] text-[13px] text-[#e8e8f0] caret-[#ff6b35] outline-none placeholder:text-[#9b59b666]"
          />
        </div>
      )}
    </div>
  )
}
