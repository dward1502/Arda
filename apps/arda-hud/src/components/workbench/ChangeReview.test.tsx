import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import ChangeReview from './ChangeReview'

describe('ChangeReview', () => {
  it('reports changed files, diffs, and test receipts', () => {
    render(<ChangeReview changes={[{ path: 'src/lib.rs', status: 'modified', additions: 2, deletions: 1, diff: '+safe boundary' }]} tests={[{ name: 'cargo test', status: 'passed', durationMs: 42 }]} />)
    expect(screen.getByText('src/lib.rs')).toBeTruthy(); expect(screen.getByText(/passed \/ 42 ms/)).toBeTruthy()
    expect(screen.getByText('+safe boundary')).toBeTruthy()
  })
  it('makes absent evidence explicit', () => {
    render(<ChangeReview changes={[]} tests={[]} />)
    expect(screen.getByText('No changes recorded.')).toBeTruthy(); expect(screen.getByText('No tests have run.')).toBeTruthy()
  })
})
