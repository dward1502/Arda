import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import WorldTerminalSurfacePreview from './WorldTerminalSurfacePreview'

vi.mock('@react-three/drei', () => ({
  Html: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}))

describe('WorldTerminalSurfacePreview display-only contract', () => {
  it('renders status without workflow controls or activation authority', () => {
    const { container } = render(
      <WorldTerminalSurfacePreview
        terminalId="terminal_status"
        layout={undefined}
        label="Status"
      />,
    )

    expect(container.querySelector('button')).toBeNull()
    expect(screen.getByText('DISPLAY ONLY')).toBeTruthy()
  })
})
