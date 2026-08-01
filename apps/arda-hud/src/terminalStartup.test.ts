// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { initializeTerminalSession } from './terminalStartup'

describe('terminal startup', () => {
  it('waits for layout and fits the PTY before reading shell output', async () => {
    const events: string[] = []

    await initializeTerminalSession({
      announce: () => events.push('announce'),
      settleLayout: async () => { events.push('settle') },
      fit: async () => { events.push('fit') },
      createShell: async () => { events.push('create-shell') },
      refresh: () => events.push('refresh'),
      focus: () => events.push('focus'),
      startReading: () => events.push('start-reading'),
    })

    expect(events).toEqual([
      'announce',
      'settle',
      'fit',
      'create-shell',
      'settle',
      'fit',
      'refresh',
      'focus',
      'start-reading',
    ])
  })
})
