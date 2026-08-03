import type { ReactNode } from 'react'
import type { VisualizationTemplateProps } from '../types'

interface StructuredData {
  node?: ReactNode
  // You can expand this later with real structured fields
  [key: string]: unknown
}

export function StructuredTemplate({
  data,
  title,
}: VisualizationTemplateProps<StructuredData | ReactNode>) {
  // Support both raw ReactNode and a small data object
  const content = data && typeof data === 'object' && 'node' in data
    ? (data as StructuredData).node
    : (data as ReactNode)

  return (
    <div className="viz-structured">
      {title && (
        <div className="viz-structured__title">
          {title}
        </div>
      )}
      <div className="viz-structured__body">
        {content ?? (
          <div style={{ color: 'var(--text-muted)', fontSize: '0.85rem' }}>
            No data
          </div>
        )}
      </div>
    </div>
  )
}