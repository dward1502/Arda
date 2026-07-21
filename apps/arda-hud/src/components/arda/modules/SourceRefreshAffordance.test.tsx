import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { SourceRefreshAffordance } from './SourceRefreshAffordance'

const baseRecord: ArdaSourceProvenance = {
  domainId: 'routing:core/state/manwe_router.json)',
  label: 'Manwe Router',
  sourcePaths: ['core/state/manwe_router.json)'],
  generatedAtUtc: '2026-06-01T00:00:00.000Z',
  observedAtUtc: null,
  state: 'stale',
  sourceKind: 'snapshot',
}

describe('SourceRefreshAffordance', () => {
  it('renders display-only projection refresh guidance', () => {
    render(<SourceRefreshAffordance record={baseRecord} />)

    expect(screen.getByText('Refresh operator runtime projection')).toBeInTheDocument()
    expect(screen.getByText('Projection refresh / display-only')).toBeInTheDocument()
    expect(screen.getByText('cargo run -p arda-cli -- utility operator-runtime-status')).toBeInTheDocument()
  })

  it('renders approval-gated queue guidance without a command', () => {
    render(
      <SourceRefreshAffordance
        record={{
          ...baseRecord,
          domainId: 'planning:core/projects/tasks/queue.jsonl',
          label: 'Task Queue',
          sourcePaths: ['core/projects/tasks/queue.jsonl'],
        }}
      />,
    )

    expect(screen.getByText('Queue refresh requires an explicit workflow')).toBeInTheDocument()
    expect(screen.getByText('Approval required / display-only')).toBeInTheDocument()
    expect(screen.queryByText(/cargo run/)).not.toBeInTheDocument()
  })

  it('hides manual-only affordances in compact mode', () => {
    const { container } = render(
      <SourceRefreshAffordance
        compact
        record={{
          ...baseRecord,
          domainId: 'world:core/state/world.json',
          label: 'World',
          sourcePaths: ['core/state/world.json'],
        }}
      />,
    )

    expect(container).toBeEmptyDOMElement()
  })
})
