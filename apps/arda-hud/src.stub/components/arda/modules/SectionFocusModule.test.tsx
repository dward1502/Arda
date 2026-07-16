// sigil: REPAIR
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import SectionFocusModule from './SectionFocusModule'

describe('SectionFocusModule', () => {
  it('renders source-map coverage warnings in the panel header', () => {
    render(
      <SectionFocusModule
        title="Knowledge And Reasoning"
        eyebrow="athena_oracle"
        accent="cyan"
        status="live"
        owner="athena_oracle"
        panelCount={2}
        sourceCount={3}
        panels={['section_focus', 'human_realm']}
        primarySources={['core/state/knowledge_triage_registry.jsonl']}
        sourceCoverage={{ status: 'partial', label: 'source map partial', missingCount: 1 }}
      />,
    )

    expect(screen.getByText('source map partial')).toBeInTheDocument()
    expect(screen.getByText('1 missing')).toBeInTheDocument()
  })

  it('renders matching source provenance freshness for primary sources', () => {
    render(
      <SectionFocusModule
        title="Routing And Comms"
        eyebrow="charon"
        accent="violet"
        status="live"
        owner="charon"
        panelCount={2}
        sourceCount={1}
        panels={['section_focus', 'operations_and_packages']}
        primarySources={['core/state/charon_router.json']}
        sourceProvenance={[
          {
            domainId: 'charon-routing',
            label: 'Charon Routing',
            sourcePaths: ['core/state/charon_router.json'],
            generatedAtUtc: '2026-06-01T00:00:00.000Z',
            observedAtUtc: null,
            state: 'fresh',
            sourceKind: 'snapshot',
          },
        ]}
      />,
    )

    expect(screen.getByText('Source Freshness')).toBeInTheDocument()
    expect(screen.getByText('Charon Routing')).toBeInTheDocument()
    expect(screen.getByText('Projection refresh')).toBeInTheDocument()
    expect(screen.getByText('Fresh')).toBeInTheDocument()
  })
})
