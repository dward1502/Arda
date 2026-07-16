// sigil: REPAIR
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import HumanRealmModule from './HumanRealmModule'

describe('HumanRealmModule', () => {
  it('renders source-map coverage in the panel header', () => {
    render(
      <HumanRealmModule
        docs={[]}
        notes={[]}
        planShelf={{ humanPlanRoot: 'human/plans', plans: [] }}
        counts={{ docs: 0, notes: 0, summaries: 0, arandur: 0 }}
        sourceCoverage={{ status: 'backed', label: 'source map backed', missingCount: 0 }}
      />,
    )

    expect(screen.getByText('source map backed')).toBeInTheDocument()
    expect(screen.getByText('0 missing')).toBeInTheDocument()
  })
})
