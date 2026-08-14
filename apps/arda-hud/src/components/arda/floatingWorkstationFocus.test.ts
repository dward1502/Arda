import { beforeEach, describe, expect, it } from 'vitest'
import {
  focusFloatingWorkstation,
  rememberFloatingWorkstationFocusOrigin,
  restoreFloatingWorkstationFocus,
} from './floatingWorkstationFocus'

describe('floating workstation focus', () => {
  beforeEach(() => {
    document.body.replaceChildren()
  })

  it('moves focus into a workstation and restores the invoking control', () => {
    const origin = document.createElement('button')
    const workstation = document.createElement('article')
    origin.textContent = 'Open settings'
    workstation.tabIndex = -1
    workstation.dataset.workstationId = 'scene-settings'
    document.body.append(origin, workstation)
    origin.focus()

    const origins = new Map<string, HTMLElement>()
    rememberFloatingWorkstationFocusOrigin(origins, 'scene-settings')

    expect(focusFloatingWorkstation('scene-settings')).toBe(true)
    expect(document.activeElement).toBe(workstation)
    expect(restoreFloatingWorkstationFocus(origins, 'scene-settings')).toBe(true)
    expect(document.activeElement).toBe(origin)
  })

  it('does not restore focus to a detached invoking control', () => {
    const origin = document.createElement('button')
    document.body.append(origin)
    origin.focus()
    const origins = new Map<string, HTMLElement>()
    rememberFloatingWorkstationFocusOrigin(origins, 'scene-settings')
    origin.remove()

    expect(restoreFloatingWorkstationFocus(origins, 'scene-settings')).toBe(false)
  })
})
