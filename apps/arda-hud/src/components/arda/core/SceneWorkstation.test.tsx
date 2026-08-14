import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import SceneWorkstation from './SceneWorkstation'

const renderWorkstation = (title: string) => render(<SceneWorkstation
  id="test-workstation"
  title={title}
  subtitle="Test"
  x={0}
  y={0}
  width={640}
  height={480}
  zIndex={1}
  modules={[{ id: 'section_focus', title: 'Overview', node: <div>Content</div> }]}
  onFocus={vi.fn()}
  onClose={vi.fn()}
  onMove={vi.fn()}
/>)

describe('SceneWorkstation', () => {
  it('adds a workstation suffix when the title does not provide one', () => {
    renderWorkstation('Settings')

    expect(screen.getByRole('dialog', { name: 'Settings workstation' })).toBeTruthy()
  })

  it('does not duplicate an existing workstation suffix', () => {
    renderWorkstation('Sovereign World Workstation')

    expect(screen.getByRole('dialog', { name: 'Sovereign World Workstation' })).toBeTruthy()
  })
})
