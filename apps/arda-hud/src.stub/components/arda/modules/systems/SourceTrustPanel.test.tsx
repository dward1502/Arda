import { render, screen } from '@testing-library/react'
import SourceTrustPanel from './SourceTrustPanel'
import type { ArdaSourceProvenance } from '../../../../lib/ardaProvenance'

const records: ArdaSourceProvenance[] = [
  {
    domainId: 'world:core/state/world.json',
    label: 'World Runtime',
    sourcePaths: ['core/state/world.json'],
    generatedAtUtc: '2026-05-22T10:00:00.000Z',
    observedAtUtc: '2026-05-22T10:00:00.000Z',
    state: 'fresh',
    sourceKind: 'snapshot',
  },
  {
    domainId: 'routing:data/prometheus/gate_matrix_last.json',
    label: 'Gate Matrix',
    sourcePaths: ['data/prometheus/gate_matrix_last.json'],
    generatedAtUtc: null,
    observedAtUtc: null,
    state: 'missing',
    sourceKind: 'snapshot',
    notes: 'Source map marks this projection missing',
  },
]

describe('SourceTrustPanel', () => {
  it('renders source provenance counts and prioritized records', () => {
    render(<SourceTrustPanel records={records} />)

    expect(screen.getByRole('region', { name: 'ARDA source trust' })).toBeInTheDocument()
    expect(screen.getByText('2')).toBeInTheDocument()
    expect(screen.getByText('fresh: 1')).toBeInTheDocument()
    expect(screen.getByText('missing: 1')).toBeInTheDocument()
    expect(screen.getByText('Gate Matrix')).toBeInTheDocument()
    expect(screen.getByText('World Runtime')).toBeInTheDocument()
  })

  it('renders an empty state when provenance has not loaded', () => {
    render(<SourceTrustPanel records={[]} />)

    expect(screen.getByText('No source provenance records available.')).toBeInTheDocument()
  })
})
