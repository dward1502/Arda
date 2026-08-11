import { act, render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import ResearchCitationDrawer from './ResearchCitationDrawer'

describe('ResearchCitationDrawer', () => {
  it('reveals canonical provenance and rejected source reasons', async () => {
    render(<ResearchCitationDrawer citations={[{ citation_id: 'cite-1', url: 'https://example.com/source', title: 'Example source', excerpt: 'bounded excerpt', fetch_status: 'rejected', rejection_reason: 'private target', quality: 'medium', freshness: 'fresh' }]} />)
    expect(screen.getByText('Example source')).toBeTruthy()
    expect(screen.getByText(/rejected · medium · fresh/)).toBeTruthy()
    await act(async () => { screen.getByRole('button', { name: /Example source/ }).click() })
    expect(await screen.findByText('private target')).toBeTruthy()
    expect(screen.getByRole('link', { name: 'Open canonical source' }).getAttribute('href')).toBe('https://example.com/source')
  })

  it('makes absent citations explicit', () => {
    render(<ResearchCitationDrawer citations={[]} />)
    expect(screen.getByText('No fetched citations are attached to this brief.')).toBeTruthy()
  })
})
