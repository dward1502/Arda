// sigil: REPAIR
import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import BusinessModule from './BusinessModule'
import ExecutiveOverviewModule from './ExecutiveOverviewModule'
import PersonalGrowthModule from './PersonalGrowthModule'
import HumanRealmModule from './HumanRealmModule'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'

const backedCoverage = { status: 'backed' as const, label: 'source map backed', missingCount: 0 }
const unmappedCoverage = { status: 'unmapped' as const, label: 'source map unmapped', missingCount: 0 }
const provenance: ArdaSourceProvenance[] = [
  {
    domainId: 'business',
    label: 'Business Runtime',
    sourcePaths: ['core/state/business_runtime.json'],
    generatedAtUtc: '2026-06-01T08:00:00Z',
    observedAtUtc: null,
    state: 'fresh',
    sourceKind: 'snapshot',
    derivedFrom: [],
  },
  {
    domainId: 'personal',
    label: 'Personal Runtime',
    sourcePaths: ['core/state/personal_runtime.json'],
    generatedAtUtc: '2026-06-01T08:01:00Z',
    observedAtUtc: null,
    state: 'derived',
    sourceKind: 'derived',
    derivedFrom: ['human/'],
  },
  {
    domainId: 'human',
    label: 'Human Context',
    sourcePaths: ['core/state/human_context.json'],
    generatedAtUtc: '2026-06-01T08:02:00Z',
    observedAtUtc: null,
    state: 'stale',
    sourceKind: 'snapshot',
    derivedFrom: [],
  },
]

describe('lower-traffic source coverage badges', () => {
  it('renders source-map coverage in executive, business, and personal module headers', () => {
    render(
      <>
        <ExecutiveOverviewModule
          authority="arda_snapshot_projection"
          loveEquation="1.00"
          activeTasks="3.00"
          schemaVersion="arda.core.state.v1"
          sourceCoverage={backedCoverage}
        />
        <BusinessModule
          mode="active"
          clientCount={2}
          stateKeyCount={4}
          companyViewTitle="Company View"
          companyViewPreview="Readable business context"
          clientPaths={['human/business/client.md']}
          stateKeys={['business.runtime']}
          sourceCoverage={unmappedCoverage}
          sourceProvenance={provenance}
        />
        <HumanRealmModule
          docs={[]}
          notes={[]}
          planShelf={{ humanPlanRoot: 'human/plans', plans: [] }}
          counts={{ docs: 1, notes: 1, summaries: 0, arandur: 0 }}
          sourceCoverage={backedCoverage}
          sourceProvenance={provenance}
        />
        <PersonalGrowthModule
          name="Daniel"
          role="Founder / Principal"
          location="Citadel"
          priorities={['autonomy']}
          values={['resonance']}
          researchDomains={['agent systems']}
          creativeDomains={['visual systems']}
          personalDocsTotal={7}
          onboardPreview="Sovereign human context"
          sourceCoverage={backedCoverage}
          sourceProvenance={provenance}
        />
      </>,
    )

    expect(screen.getAllByText('source map backed')).toHaveLength(3)
    expect(screen.getByText('source map unmapped')).toBeInTheDocument()
    expect(screen.getByLabelText('Business Source Freshness')).toBeInTheDocument()
    expect(screen.getByLabelText('Human Source Freshness')).toBeInTheDocument()
    expect(screen.getByLabelText('Personal Source Freshness')).toBeInTheDocument()
    expect(within(screen.getByLabelText('Business Source Freshness')).getByText('Business Runtime')).toBeInTheDocument()
    expect(within(screen.getByLabelText('Human Source Freshness')).getByText('Human Context')).toBeInTheDocument()
    expect(within(screen.getByLabelText('Personal Source Freshness')).getByText('Personal Runtime')).toBeInTheDocument()
  })
})
