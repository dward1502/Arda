import { render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import ResearchModule from './ResearchModule'

const fetchMock = vi.fn()

beforeEach(() => {
  fetchMock.mockReset()
  fetchMock.mockImplementation((input: RequestInfo | URL) => {
    const url = String(input)
    const body = url.endsWith('/questions') ? { questions: [] } : url.endsWith('/watchlists') ? { watchlists: [] } : { briefs: [] }
    return Promise.resolve(new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } }))
  })
  vi.stubGlobal('fetch', fetchMock)
})

describe('ResearchModule', () => {
  it('exposes explicit question and bounded watchlist flows before shell wiring', async () => {
    render(<ResearchModule />)
    expect(await screen.findByRole('heading', { name: 'Compose explicit question' })).toBeTruthy()
    expect(screen.getByRole('textbox', { name: 'Question' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Create bounded question' })).toBeTruthy()
    expect(screen.getByRole('heading', { name: 'Compose watchlist' })).toBeTruthy()
    expect(screen.getByText('Warden → Varda → advisory brief')).toBeTruthy()
    expect(fetchMock).toHaveBeenCalledTimes(3)
  })
})
