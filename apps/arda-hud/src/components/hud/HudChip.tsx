import type { HTMLAttributes } from 'react'

function classes(...values: Array<string | undefined>): string {
  return values.filter(Boolean).join(' ')
}

type ChipAccent = 'cyan' | 'orange' | 'danger' | 'gold'

export interface HudChipProps extends HTMLAttributes<HTMLSpanElement> {
  accent?: ChipAccent
}

export function HudChip({
  accent = 'cyan',
  className,
  children,
  ...rest
}: HudChipProps) {
  return (
    <span
      className={classes('hud-chip', `hud-chip--${accent}`, className)}
      {...rest}
    >
      {children}
    </span>
  )
}