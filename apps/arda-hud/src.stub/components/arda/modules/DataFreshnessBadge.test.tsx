import React from 'react'
import { render, screen } from '@testing-library/react'
import { DataFreshnessBadge } from './DataFreshnessBadge'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'

const mockProvenanceRecord: ArdaSourceProvenance = {
  domainId: 'test-domain',
  label: 'Test Domain',
  sourcePaths: ['test/source/path.json'],
  generatedAtUtc: '2026-05-22T10:00:00.000Z',
  observedAtUtc: '2026-05-22T10:00:00.000Z',
  state: 'fresh',
  sourceKind: 'snapshot',
  derivedFrom: [],
  safeRefreshCommand: 'refresh-command',
  lastRefreshResult: { success: true, message: 'Success', timestampUtc: '2026-05-22T10:00:00.000Z' },
  notes: 'Test notes'
}

describe('DataFreshnessBadge', () => {
  test('renders with fresh state in compact mode', () => {
    const { container } = render(
      <DataFreshnessBadge record={mockProvenanceRecord} compact={true} />
    )
    
    expect(screen.getByText('Fresh')).toBeInTheDocument()
    expect(container).toMatchSnapshot()
  })

  test('renders with stale state in expanded mode', () => {
    const staleRecord = { ...mockProvenanceRecord, state: 'stale' as const }
    render(<DataFreshnessBadge record={staleRecord} compact={false} />)
    
    expect(screen.getByText('Stale')).toBeInTheDocument()
  })

  test('renders with missing state', () => {
    const missingRecord = { ...mockProvenanceRecord, state: 'missing' as const }
    render(<DataFreshnessBadge record={missingRecord} />)
    
    expect(screen.getByText('Missing')).toBeInTheDocument()
  })

  test('renders with derived state', () => {
    const derivedRecord = { ...mockProvenanceRecord, state: 'derived' as const }
    render(<DataFreshnessBadge record={derivedRecord} />)
    
    expect(screen.getByText('Derived')).toBeInTheDocument()
  })

  test('renders with blocked state', () => {
    const blockedRecord = { ...mockProvenanceRecord, state: 'blocked' as const }
    render(<DataFreshnessBadge record={blockedRecord} />)
    
    expect(screen.getByText('Blocked')).toBeInTheDocument()
  })

  test('renders with unknown state', () => {
    const unknownRecord = { ...mockProvenanceRecord, state: 'unknown' as const }
    render(<DataFreshnessBadge record={unknownRecord} />)
    
    expect(screen.getByText('Unknown')).toBeInTheDocument()
  })

  test('shows source paths in tooltip', async () => {
    const { container } = render(
      <DataFreshnessBadge record={mockProvenanceRecord} />
    )
    
    // Simulate hover
    const badge = container.querySelector('[title]')
    expect(badge).toHaveAttribute('title', expect.stringContaining('test/source/path.json'))
  })
})
