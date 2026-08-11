import type { ReactNode, HTMLAttributes } from 'react'

interface HudFrameProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  header?: ReactNode
  status?: ReactNode
  children?: ReactNode
  /** Stroke color – defaults to cyan */
  color?: string
  /** Frame thickness */
  strokeWidth?: number
}

export function HudFrame({
  header,
  status,
  children,
  color = 'rgba(0, 212, 255, 0.75)',
  strokeWidth = 1.5,
  className = '',
  style,
  ...rest
}: HudFrameProps) {
  return (
    <div
      className={`hud-frame ${className}`.trim()}
      style={{
        position: 'relative',
        ...style,
      }}
      {...rest}
    >
      {/* SVG Frame */}
      <svg
        className="hud-frame__svg"
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
        style={{
          position: 'absolute',
          inset: 0,
          width: '100%',
          height: '100%',
          pointerEvents: 'none',
          overflow: 'visible',
        }}
      >
        {/* Outer cut-corner frame */}
        <path
          d="
            M 8 0
            L 92 0
            L 100 8
            L 100 92
            L 92 100
            L 8 100
            L 0 92
            L 0 8
            Z
          "
          fill="rgba(3, 8, 14, 0.92)"
          stroke={color}
          strokeWidth={strokeWidth}
          vectorEffect="non-scaling-stroke"
        />

        {/* Inner content border */}
        <path
          d="
            M 10 3
            L 90 3
            L 97 10
            L 97 90
            L 90 97
            L 10 97
            L 3 90
            L 3 10
            Z
          "
          fill="none"
          stroke={color}
          strokeWidth={strokeWidth * 0.6}
          strokeOpacity={0.35}
          vectorEffect="non-scaling-stroke"
        />

        {/* Top-left bracket */}
        <path
          d="M 0 14 L 0 0 L 14 0"
          fill="none"
          stroke={color}
          strokeWidth={strokeWidth * 1.8}
          strokeLinecap="square"
          vectorEffect="non-scaling-stroke"
        />

        {/* Bottom-right bracket */}
        <path
          d="M 100 86 L 100 100 L 86 100"
          fill="none"
          stroke={color}
          strokeWidth={strokeWidth * 1.8}
          strokeLinecap="square"
          vectorEffect="non-scaling-stroke"
        />
      </svg>

      {/* Content */}
      <div
        className="hud-frame__content"
        style={{
          position: 'relative',
          zIndex: 1,
          height: '100%',
          display: 'flex',
          flexDirection: 'column',
          padding: '2px',
        }}
      >
        {(header || status) && (
          <div
            className="hud-frame__header"
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '0.4rem 0.75rem',
              borderBottom: `1px solid ${color}33`,
              fontFamily: 'var(--title-font)',
              fontSize: '0.62rem',
              letterSpacing: '0.14em',
              textTransform: 'uppercase',
              color: 'rgba(180, 220, 255, 0.75)',
            }}
          >
            {header && (
              <span style={{ color: '#e2f4ff', fontWeight: 600 }}>{header}</span>
            )}
            {status}
          </div>
        )}

        <div
          className="hud-frame__body"
          style={{
            flex: 1,
            minHeight: 0,
            overflow: 'auto',
            padding: '0.6rem 0.75rem',
          }}
        >
          {children}
        </div>
      </div>
    </div>
  )
}