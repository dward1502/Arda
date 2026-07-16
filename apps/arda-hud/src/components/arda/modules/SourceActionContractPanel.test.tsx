// sigil: REPAIR
import { fireEvent, render, screen, within } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import SourceActionContractPanel from './SourceActionContractPanel'

const record: ArdaSourceProvenance = {
  domainId: 'systems:charon',
  label: 'Charon Router',
  sourcePaths: ['core/state/charon_router.json'],
  generatedAtUtc: '2026-06-01T07:00:00Z',
  observedAtUtc: '2026-06-01T07:01:00Z',
  state: 'fresh',
  sourceKind: 'snapshot',
  derivedFrom: [],
  notes: 'Provider routing projection.',
}

describe('SourceActionContractPanel', () => {
  beforeEach(() => {
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    })
  })

  it('renders safe copy/reveal actions and gated share action', () => {
    render(<SourceActionContractPanel record={record} />)

    const panel = screen.getByLabelText('Source action contract')
    expect(within(panel).getByText('Source Action Contract')).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Copy Source Paths/ })).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Copy Source Packet/ })).toBeTruthy()
    expect(within(panel).getByRole('button', { name: /Reveal Native Source/ })).toBeTruthy()
    expect(within(panel).getByText(/Share Source Packet: gated/)).toBeTruthy()
  })

  it('copies source paths to the clipboard', async () => {
    render(<SourceActionContractPanel record={record} />)

    fireEvent.click(screen.getByRole('button', { name: /Copy Source Paths/ }))

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('core/state/charon_router.json')
    expect(await screen.findByText('copy_source_paths copied')).toBeTruthy()
  })
})
