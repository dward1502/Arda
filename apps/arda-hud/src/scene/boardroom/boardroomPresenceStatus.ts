import type { AgentPresenceState, PresenceLedgerStatus } from '../systems/presenceTypes'

export interface BoardroomPresenceStatusView {
  label: string
  detail: string
  className: string
  title: string
}

function observedTime(timestamp: string | undefined): string {
  return timestamp?.slice(11, 16) ?? '--:--'
}

export function deriveBoardroomPresenceStatusView(
  status: PresenceLedgerStatus | undefined,
  state: AgentPresenceState,
): BoardroomPresenceStatusView {
  if (!status) {
    return {
      label: 'Presence fallback',
      detail: 'missing --:--Z · Default ARDA state',
      className: 'presence-ledger-status presence-ledger-status--fallback',
      title: 'Presence ledger status unavailable',
    }
  }

  const title = `${status.sourcePath} · observed ${status.latestTimestamp ?? 'unknown'} · ${status.summary}`
  if (status.source === 'fallback_default') {
    const malformed = status.malformedLineCount > 0
      ? ` · ${status.malformedLineCount} malformed`
      : ''
    return {
      label: 'Presence fallback',
      detail: `missing --:--Z${malformed} · Default ARDA state`,
      className: 'presence-ledger-status presence-ledger-status--fallback',
      title,
    }
  }

  const agent = state.primaryAgent.toUpperCase()
  return {
    label: status.freshness === 'fresh' ? 'Presence live' : 'Presence stale',
    detail: `${agent} · ${observedTime(status.latestTimestamp)}Z · ${status.freshness} · ${status.validEventCount} ledger row${status.validEventCount === 1 ? '' : 's'}`,
    className: `presence-ledger-status presence-ledger-status--${status.freshness}`,
    title,
  }
}
