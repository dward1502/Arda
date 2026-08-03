import type { ReactNode, HTMLAttributes } from 'react'

function classes(...values: Array<string | false | undefined>): string {
  return values.filter(Boolean).join(' ')
}

type HudAccent = 'cyan' | 'orange' | 'purple' | 'danger' | 'gold'
type HudVariant = 'default' | 'strong' | 'ghost'

export interface HudPanelProps extends HTMLAttributes<HTMLDivElement> {
  accent?: HudAccent
  variant?: HudVariant
  cut?: boolean
  glow?: boolean
  header?: ReactNode
  status?: ReactNode
  children?: ReactNode
}

export function HudPanel({
  accent = 'cyan',
  variant = 'default',
  cut = true,
  glow = false,
  header,
  status,
  className,
  children,
  ...rest
}: HudPanelProps) {
  return (
    <div
      className={classes(
        'hud-panel',
        cut && 'hud-panel--cut',
        accent && `hud-panel--${accent}`,
        variant === 'strong' && 'hud-panel--strong',
        glow && 'hud-panel--glow',
        className,
      )}
      {...rest}
    >
      {(header || status) && (
        <div className="hud-header">
          <div className="hud-header__title">{header}</div>
          {status && <div className="hud-header__status">{status}</div>}
        </div>
      )}
      <div className="hud-panel__body">{children}</div>
    </div>
  )
}