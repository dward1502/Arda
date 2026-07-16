// sigil: REPAIR
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import RoutableProvidersPanel from './RoutableProvidersPanel'

describe('RoutableProvidersPanel', () => {
  it('renders direct Charon router provider and model health details', () => {
    render(
      <RoutableProvidersPanel
        providers={[
          {
            providerId: 'opencode',
            providerName: 'OpenCode Zen',
            accessTier: 'free_cloud',
            qualityBand: 'high',
            enabled: true,
            healthy: true,
            avgLatencyMs: 42,
            activeConnections: 2,
            models: [
              {
                id: 'opencode/deepseek-v4-flash-free',
                contextWindow: 128000,
                healthy: true,
                isDefault: true,
                capableTasks: ['code', 'research'],
              },
            ],
          },
        ]}
      />,
    )

    expect(screen.getByRole('heading', { name: /Routable Providers/i })).toBeInTheDocument()
    expect(screen.getByText('OpenCode Zen')).toBeInTheDocument()
    expect(screen.getAllByText(/opencode/).length).toBeGreaterThanOrEqual(1)
    expect(screen.getByText(/free_cloud/)).toBeInTheDocument()
    expect(screen.getByText(/high/)).toBeInTheDocument()
    expect(screen.getAllByText(/healthy/).length).toBeGreaterThanOrEqual(1)
    expect(screen.getByText(/opencode\/deepseek-v4-flash-free/)).toBeInTheDocument()
    expect(screen.getByText(/128k ctx/)).toBeInTheDocument()
    expect(screen.getByText(/default/)).toBeInTheDocument()
    expect(screen.getByText(/code, research/)).toBeInTheDocument()
  })
})
