// sigil: REPAIR
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { CharonLiveSnapshot } from '../../../../lib/charonLive'
import CharonCapabilityPanel from './CharonCapabilityPanel'

const liveSnapshot: CharonLiveSnapshot = {
  loadedAt: '2026-06-05T12:00:00Z',
  capabilities: {
    ok: true,
    capabilities: {
      schema_version: '1',
      generated_at_utc: '2026-06-05T12:00:00Z',
      summary: {
        receipt_model_count: 6,
        models_with_failed_tool_receipts: 1,
        models_with_failed_structured_output_receipts: 0,
        models_with_failed_streaming_receipts: 2,
        recent_capability_failures: 1,
        providers_with_no_capability_evidence: 4,
      },
      providers: [
        {
          provider_id: 'mistral',
          enabled: true,
          access_tier: 'mixed',
          evidence_state: 'observed',
          models: [],
        },
        {
          provider_id: 'nvidia',
          enabled: true,
          access_tier: 'mixed',
          evidence_state: 'unknown',
          models: [],
        },
      ],
    },
  },
  providerCandidates: {
    ok: true,
    promotion_guard: {
      schema_version: '1',
      generated_at_utc: '2026-06-05T12:00:00Z',
      active_capability_probes_enabled: false,
      candidates: [
        {
          id: 'together',
          name: 'Together AI',
          status: 'candidate',
          free_kind: 'credits',
          access_tier_candidate: 'free_credits',
          requires_adapter: true,
          promotion_ready: false,
          reasons: ['adapter_required'],
        },
        {
          id: 'edge_guardhouse',
          name: 'Edge Guardhouse',
          status: 'enabled',
          free_kind: 'community',
          access_tier_candidate: 'free_cloud',
          requires_adapter: false,
          promotion_ready: true,
          reasons: [],
        },
      ],
    },
  },
}

describe('CharonCapabilityPanel', () => {
  it('renders live capability receipts and candidate guard state', () => {
    render(<CharonCapabilityPanel snapshot={liveSnapshot} error={null} loading={false} />)

    expect(screen.getByRole('heading', { name: /Model Receipts \+ Promotion Guard/i })).toBeInTheDocument()
    expect(screen.getByText('observed')).toBeInTheDocument()
    expect(screen.getByText('6')).toBeInTheDocument()
    expect(screen.getByText('1')).toBeInTheDocument()
    expect(screen.getByText('4')).toBeInTheDocument()
    expect(screen.getByText(/tool failed 1/i)).toBeInTheDocument()
    expect(screen.getByText(/stream failed 2/i)).toBeInTheDocument()
    expect(screen.getByText(/active probes off/i)).toBeInTheDocument()
    expect(screen.getByText(/mistral observed/i)).toBeInTheDocument()
    expect(screen.getByText(/nvidia unknown/i)).toBeInTheDocument()
    expect(screen.getByText(/together adapter/i)).toBeInTheDocument()
    expect(screen.getByText(/edge_guardhouse ready/i)).toBeInTheDocument()
  })

  it('shows the operator-facing error when the Charon stream is unavailable', () => {
    render(<CharonCapabilityPanel snapshot={null} error="Charon /provider_candidates returned 502" loading={false} />)

    expect(screen.getByText('blocked')).toBeInTheDocument()
    expect(screen.getByText('Charon /provider_candidates returned 502')).toBeInTheDocument()
    expect(screen.getByText('No live provider evidence yet.')).toBeInTheDocument()
    expect(screen.getByText('No provider candidates loaded.')).toBeInTheDocument()
  })
})
