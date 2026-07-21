// sigil: REPAIR
import { Html } from '@react-three/drei'
import type { CSSProperties } from 'react'
import type { WorldSurfaceLayout } from '../../lib/worldSurfaceSettings'
import type { BoardroomSurfacePreviewWidgetModel } from '../boardroom/boardroomSurfacePreviewModel'
import { deriveWorldTerminalSurfacePreviewModel } from './worldTerminalSurfacePreviewModel'

function TerminalStatusGrid({ widget }: { widget: BoardroomSurfacePreviewWidgetModel }) {
  return (
    <span className="world-terminal-preview__status-grid">
      {widget.values.map((value, index) => (
        <i
          key={`${widget.id}-status-${index}`}
          className={value > 0.72 ? 'is-hot' : value > 0.44 ? 'is-watch' : 'is-ready'}
        />
      ))}
    </span>
  )
}

function TerminalFeed({ widget }: { widget: BoardroomSurfacePreviewWidgetModel }) {
  return (
    <span className="world-terminal-preview__feed">
      <b>{widget.title}</b>
      {widget.values.slice(0, 3).map((value, index) => (
        <i key={`${widget.id}-feed-${index}`} style={{ '--feed-level': `${Math.max(0.18, value) * 100}%` } as CSSProperties}>
          {widget.binding}
        </i>
      ))}
    </span>
  )
}

function TerminalWidget({ widget }: { widget: BoardroomSurfacePreviewWidgetModel }) {
  if (widget.kind === 'status_grid') return <TerminalStatusGrid widget={widget} />
  return <TerminalFeed widget={widget} />
}

export default function WorldTerminalSurfacePreview({
  terminalId,
  layout,
  label,
  onActivate,
}: {
  terminalId: string
  layout: WorldSurfaceLayout | undefined
  label: string
  onActivate: () => void
}) {
  const model = deriveWorldTerminalSurfacePreviewModel({ terminalId, layout })
  const className = [
    'world-terminal-preview',
    `world-terminal-preview--${model.tone}`,
    `world-terminal-preview--${model.status}`,
  ].join(' ')

  return (
    <Html center distanceFactor={9}>
      <button type="button" className={className} onClick={onActivate} aria-label={`Open ${model.title}`}>
        <span className="world-terminal-preview__header">
          <b>{label}</b>
          <i>{model.glyph}</i>
        </span>
        <strong>{model.title}</strong>
        <span className="world-terminal-preview__widgets">
          {model.widgets.slice(0, 2).map((widget) => (
            <TerminalWidget key={widget.id} widget={widget} />
          ))}
        </span>
        <span className="world-terminal-preview__footer">
          <span>{model.safeActionSummary}</span>
          <small>{model.focusMode.replace(/_/g, ' ')}</small>
        </span>
      </button>
    </Html>
  )
}
