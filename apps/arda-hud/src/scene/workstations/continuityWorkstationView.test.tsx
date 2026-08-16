import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { ContinuityViewModel } from './viewModels'
import { ContinuityFocusedWorkstationView } from './continuityWorkstationView'

const model: ContinuityViewModel = {
  roleId: 'continuity',
  title: 'Human + Business + Personal',
  status: 'attention',
  summary: ['continuity summary'],
  metrics: [],
  sources: [
    { id: 'human', label: 'Human Context', freshness: { status: 'snapshot' } },
    { id: 'company', label: 'Company Operations', freshness: { status: 'unavailable' } },
  ],
  actions: [],
  previewKinds: [],
  focusedCapabilities: [],
  rawDisclosure: false,
  horizons: [
    { id: 'human', label: 'Human', count: 1, attention: 0 },
    { id: 'business', label: 'Business', count: 2, attention: 1 },
    { id: 'personal', label: 'Personal', count: 1, attention: 0 },
  ],
  items: [
    { id: 'human:1', horizon: 'human', kind: 'note', title: 'Current context', summary: 'Readable context', state: 'snapshot', privateDetail: true },
    { id: 'business:1', horizon: 'business', kind: 'client', title: 'Missing client', summary: 'Path absent', state: 'missing', path: 'missing.json', privateDetail: false },
    { id: 'business:2', horizon: 'business', kind: 'offer', title: 'Planned offer', summary: 'Forecast only', state: 'planned', privateDetail: false },
    { id: 'personal:1', horizon: 'personal', kind: 'priority', title: 'Family continuity', summary: 'Private detail', state: 'active', privateDetail: true },
  ],
  valueTruth: { plannedMinor: 50000, realizedMinor: 0, currency: 'USD', realizedReceiptCount: 0 },
  missingReferenceCount: 1,
}

describe('ContinuityFocusedWorkstationView', () => {
  it('separates horizons, value truth, missing paths, and private detail on the focused surface', () => {
    render(<ContinuityFocusedWorkstationView model={model} />)

    expect(screen.getByRole('heading', { name: 'Human + Business + Personal' })).toBeInTheDocument()
    expect(screen.getByText('Planned value')).toBeInTheDocument()
    expect(screen.getByText('$500.00')).toBeInTheDocument()
    expect(screen.getByText('No receipt-backed realized value')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: /Business 2/i }))
    expect(screen.getByRole('button', { name: /Missing client/i })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /Missing client/i }))
    expect(screen.getByText('missing.json')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: /Personal 1/i }))
    expect(screen.getByText('Private focused detail')).toBeInTheDocument()
  })

  it('falls back to the first remaining record when the selected collection changes', () => {
    const { rerender } = render(<ContinuityFocusedWorkstationView model={model} />)
    fireEvent.click(screen.getByRole('button', { name: /Business 2/i }))
    fireEvent.click(screen.getByRole('button', { name: /Planned offer/i }))
    expect(screen.getByRole('heading', { name: 'Planned offer' })).toBeInTheDocument()

    rerender(<ContinuityFocusedWorkstationView model={{
      ...model,
      horizons: model.horizons.map((horizon) => horizon.id === 'business' ? { ...horizon, count: 1 } : horizon),
      items: model.items.filter((item) => item.id !== 'business:2'),
    }} />)

    expect(screen.getByRole('heading', { name: 'Missing client' })).toBeInTheDocument()
  })
})
