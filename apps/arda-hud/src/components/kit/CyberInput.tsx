// sigil: REPAIR
import { forwardRef } from 'react'
import type { ReactNode } from 'react'

type InputValidation = 'default' | 'error' | 'success'

interface CyberInputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'className'> {
  validation?: InputValidation
  className?: string
}

interface CyberTextareaProps extends Omit<React.TextareaHTMLAttributes<HTMLTextAreaElement>, 'className'> {
  validation?: InputValidation
  className?: string
}

interface CyberSelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'className'> {
  validation?: InputValidation
  className?: string
}

interface InputGroupProps {
  label: string
  children: ReactNode
  className?: string
}

const baseInputClasses =
  "w-full bg-[#140020] border border-[#9b59b666] border-b-[#ff6b3566] px-3.5 py-2.5 font-['Share_Tech_Mono'] text-[13px] text-[#e8e8f0] outline-none transition-all duration-200 placeholder:text-[#9b59b666] placeholder:font-['Share_Tech_Mono'] focus:border-[#ff6b35] focus:border-b-[#ff6b35] focus:shadow-[0_2px_0_#ff6b35,inset_0_0_20px_rgba(255,107,53,0.13)]"

const validationClasses: Record<InputValidation, string> = {
  default: '',
  error: 'border-[#ff3333] shadow-[0_0_8px_rgba(255,51,51,0.13)]',
  success: 'border-[#f9ca24]',
}

const SELECT_ARROW =
  "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8' viewBox='0 0 12 8'%3E%3Cpath d='M1 1l5 5 5-5' stroke='%23ff6b35' stroke-width='1.5' fill='none'/%3E%3C/svg%3E\")"

const CyberInput = forwardRef<HTMLInputElement, CyberInputProps>(
  function CyberInput({ validation = 'default', className = '', ...props }, ref) {
    return (
      <input
        ref={ref}
        className={`${baseInputClasses} ${validationClasses[validation]} ${className}`}
        {...props}
      />
    )
  }
)

export default CyberInput

export const CyberTextarea = forwardRef<HTMLTextAreaElement, CyberTextareaProps>(
  function CyberTextarea({ validation = 'default', className = '', ...props }, ref) {
    return (
      <textarea
        ref={ref}
        className={`${baseInputClasses} min-h-20 resize-y leading-relaxed ${validationClasses[validation]} ${className}`}
        {...props}
      />
    )
  }
)

export const CyberSelect = forwardRef<HTMLSelectElement, CyberSelectProps>(
  function CyberSelect({ validation = 'default', className = '', children, ...props }, ref) {
    return (
      <select
        ref={ref}
        className={`${baseInputClasses} cursor-pointer appearance-none bg-no-repeat pr-9 ${validationClasses[validation]} ${className}`}
        style={{
          backgroundImage: SELECT_ARROW,
          backgroundPosition: 'right 14px center',
        }}
        {...props}
      >
        {children}
      </select>
    )
  }
)

export function InputGroup({ label, children, className = '' }: InputGroupProps) {
  return (
    <div className={`mb-4 flex flex-col gap-1.5 ${className}`}>
      <label className="font-['Orbitron'] text-[8px] uppercase tracking-[0.18em] text-[#9b59b6]">
        {label}
      </label>
      {children}
    </div>
  )
}
