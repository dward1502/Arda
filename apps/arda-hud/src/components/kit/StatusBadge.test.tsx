// sigil: REPAIR
import { render, screen } from '@testing-library/react'
import StatusBadge from './StatusBadge'

describe('StatusBadge', () => {
  it('renders label text', () => {
    render(<StatusBadge state="info" label="Peers 3" />)
    expect(screen.getByText('Peers 3')).toBeInTheDocument()
  })

  it('applies warning state classes', () => {
    render(<StatusBadge state="warning" label="Degraded" />)
    expect(screen.getByText('Degraded')).toHaveClass('text-[#f9ca24]')
  })
})
