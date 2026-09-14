import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import GlobalRapidCapture from './GlobalRapidCapture'
import type { PersonalOpsClient } from '../../lib/personalOps'

function captureClient(createCapture = vi.fn(async () => ({
  event_id: 'event-1',
  capture_id: 'capture-1',
}))): Pick<PersonalOpsClient, 'createCapture'> {
  return { createCapture }
}

describe('GlobalRapidCapture', () => {
  it('opens from the HUD shortcut and focuses the capture field', async () => {
    render(<GlobalRapidCapture client={captureClient()} operatorId="operator" />)

    fireEvent.keyDown(window, { key: ' ', code: 'Space', ctrlKey: true, shiftKey: true })

    expect(screen.getByRole('dialog', { name: 'Rapid capture' })).toBeInTheDocument()
    await waitFor(() => expect(screen.getByLabelText('Capture a thought')).toHaveFocus())
    expect(screen.getByText('Ctrl+Shift+Space')).toBeInTheDocument()
  })

  it('saves operator-authored text and shows durable confirmation', async () => {
    const createCapture = vi.fn(async () => ({ event_id: 'event-1', capture_id: 'capture-1' }))
    const saved = vi.fn()
    render(
      <GlobalRapidCapture
        client={captureClient(createCapture)}
        operatorId="operator"
        onSaved={saved}
      />,
    )
    fireEvent.keyDown(window, { key: ' ', code: 'Space', ctrlKey: true, shiftKey: true })
    const input = screen.getByLabelText('Capture a thought')
    fireEvent.change(input, { target: { value: 'well lets complete phase 4' } })

    fireEvent.click(screen.getByRole('button', { name: 'Save capture' }))

    await waitFor(() => expect(createCapture).toHaveBeenCalledWith('well lets complete phase 4'))
    expect(await screen.findByRole('status')).toHaveTextContent('Capture saved durably')
    expect(input).toHaveValue('')
    expect(saved).toHaveBeenCalledWith({ event_id: 'event-1', capture_id: 'capture-1' })
  })

  it('keeps unsaved text visible when the backend rejects the capture', async () => {
    const createCapture = vi.fn(async () => {
      throw new Error('personal operations unavailable')
    })
    render(<GlobalRapidCapture client={captureClient(createCapture)} operatorId="operator" />)
    fireEvent.keyDown(window, { key: ' ', code: 'Space', ctrlKey: true, shiftKey: true })
    const input = screen.getByLabelText('Capture a thought')
    fireEvent.change(input, { target: { value: 'keep this thought' } })

    fireEvent.click(screen.getByRole('button', { name: 'Save capture' }))

    expect(await screen.findByRole('alert')).toHaveTextContent('personal operations unavailable')
    expect(input).toHaveValue('keep this thought')
  })
})
