import { useCallback, useEffect, useMemo, useState, type KeyboardEvent } from 'react'
import { createPersonalOpsClient, loadConfiguredOperatorId, type PersonalOpsClient, type PersonalOpsSnapshot } from '../../../lib/personalOps'
import ModuleCard from '../ModuleCard'

interface PersonalOperationsModuleProps {
  client?: PersonalOpsClient
  operatorId?: string
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function formatTime(value: string | null): string {
  if (!value) return 'unscheduled'
  const instant = new Date(value)
  return Number.isNaN(instant.valueOf()) ? value : instant.toLocaleString()
}

export default function PersonalOperationsModule({
  client,
  operatorId,
}: PersonalOperationsModuleProps) {
  const [configuredOperatorId, setConfiguredOperatorId] = useState<string | null>(() => operatorId?.trim() || null)
  const defaultClient = useMemo(
    () => configuredOperatorId ? createPersonalOpsClient(configuredOperatorId) : null,
    [configuredOperatorId],
  )
  const ops = client ?? defaultClient
  const [snapshot, setSnapshot] = useState<PersonalOpsSnapshot | null>(null)
  const [capture, setCapture] = useState('')
  const [busy, setBusy] = useState(false)
  const [selectedReviewIds, setSelectedReviewIds] = useState<Set<string>>(new Set())
  const [inboxKinds, setInboxKinds] = useState<Record<string, string>>({})
  const [scheduleDrafts, setScheduleDrafts] = useState<Record<string, string>>({})
  const [deletePending, setDeletePending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [status, setStatus] = useState(client || configuredOperatorId
    ? 'Loading personal operations'
    : 'Resolving configured operator identity')

  useEffect(() => {
    if (client || configuredOperatorId) return
    let cancelled = false
    void loadConfiguredOperatorId().then((identity) => {
      if (cancelled) return
      setConfiguredOperatorId(identity)
      setStatus('Loading personal operations')
    }).catch((caught) => {
      if (cancelled) return
      setError(messageOf(caught))
      setStatus('Personal operations unavailable')
    })
    return () => { cancelled = true }
  }, [client, configuredOperatorId])

  const refresh = useCallback(async () => {
    if (!ops) throw new Error('Configured operator identity is unavailable')
    const next = await ops.loadSnapshot()
    setSnapshot(next)
    setStatus('Personal operations loaded')
    setError(null)
  }, [ops])

  useEffect(() => {
    if (!ops) return
    let cancelled = false
    void ops.loadSnapshot().then((next) => {
      if (cancelled) return
      setSnapshot(next)
      setStatus('Personal operations loaded')
    }).catch((caught) => {
      if (cancelled) return
      setError(messageOf(caught))
      setStatus('Personal operations unavailable')
    })
    return () => { cancelled = true }
  }, [ops])

  const submitCapture = async () => {
    const text = capture.trim()
    if (!text || busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      await ops.createCapture(text)
      setCapture('')
      setStatus('Capture saved')
      await refresh()
    } catch (caught) {
      setError(messageOf(caught))
      setStatus('Capture failed')
    } finally {
      setBusy(false)
    }
  }

  const onCaptureKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== 'Enter' || event.shiftKey) return
    event.preventDefault()
    void submitCapture()
  }

  const acknowledge = async (reminderId: string) => {
    if (busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      await ops.acknowledgeReminder(reminderId)
      setStatus('Reminder acknowledged')
      await refresh()
    } catch (caught) {
      setError(messageOf(caught))
      setStatus('Reminder acknowledgement failed')
    } finally {
      setBusy(false)
    }
  }

  const respondToReminder = async (reminderId: string, state: 'deferred' | 'dismissed') => {
    if (busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      await ops.respondToReminder(reminderId, state)
      setStatus(`Reminder ${state}`)
      await refresh()
    } catch (caught) {
      setError(messageOf(caught))
      setStatus(`Reminder ${state} action failed`)
    } finally {
      setBusy(false)
    }
  }

  const classifyInboxItem = async (itemId: string) => {
    if (busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      await ops.confirmClassification(itemId, inboxKinds[itemId] ?? 'task')
      setStatus('Inbox item classified')
      await refresh()
    } catch (caught) {
      setError(messageOf(caught))
      setStatus('Inbox classification failed')
    } finally {
      setBusy(false)
    }
  }

  const completeItem = async (itemId: string) => {
    if (busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      await ops.completeItem(itemId)
      setStatus('Item completed')
      await refresh()
    } catch (caught) {
      setError(messageOf(caught))
      setStatus('Completion failed')
    } finally {
      setBusy(false)
    }
  }

  const scheduleItem = async (itemId: string) => {
    const draft = scheduleDrafts[itemId]
    if (!draft || busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      await ops.scheduleItem(itemId, new Date(draft).toISOString())
      setStatus('Item scheduled')
      await refresh()
    } catch (caught) {
      setError(messageOf(caught))
      setStatus('Scheduling failed')
    } finally {
      setBusy(false)
    }
  }

  const brief = snapshot?.todayBrief.brief
  const today = brief?.today ?? []
  const waiting = brief?.waiting ?? []
  const inbox = snapshot?.inbox.inbox ?? []
  const nextAction = today[0] ?? waiting[0] ?? null
  const reviewCandidates = [...today, ...waiting]
    .filter((item, index, items) => item.evidence_class !== 'operator_authored'
      && items.findIndex((candidate) => candidate.item_id === item.item_id) === index)

  const confirmSelected = async () => {
    const selected = reviewCandidates.filter((item) => selectedReviewIds.has(item.item_id)).slice(0, 10)
    if (selected.length === 0 || busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      for (const item of selected) await ops.confirmClassification(item.item_id, item.kind)
      setSelectedReviewIds(new Set())
      setStatus(`${selected.length} classification${selected.length === 1 ? '' : 's'} confirmed`)
      await refresh()
    } catch (caught) {
      setError(messageOf(caught))
      setStatus('Classification review failed')
    } finally {
      setBusy(false)
    }
  }

  const exportPersonalData = async () => {
    if (busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      const exported = await ops.exportPersonalData()
      const href = URL.createObjectURL(new Blob([JSON.stringify(exported, null, 2)], { type: 'application/json' }))
      const link = document.createElement('a')
      link.href = href
      link.download = `arda-personal-data-${exported.generated_at.slice(0, 10)}.json`
      link.click()
      URL.revokeObjectURL(href)
      setStatus(`Personal data export ready with ${exported.events.length} event${exported.events.length === 1 ? '' : 's'}`)
    } catch (caught) {
      setError(messageOf(caught))
      setStatus('Personal data export failed')
    } finally {
      setBusy(false)
    }
  }

  const deletePersonalData = async () => {
    if (busy || !ops) return
    setBusy(true)
    setError(null)
    try {
      const result = await ops.deletePersonalData()
      setDeletePending(false)
      await refresh()
      setStatus(`Deleted ${result.deleted_events} personal event${result.deleted_events === 1 ? '' : 's'}; system receipts preserved`)
    } catch (caught) {
      setError(messageOf(caught))
      setStatus('Personal data deletion failed')
    } finally {
      setBusy(false)
    }
  }

  return (
    <ModuleCard
      title="Personal Operations"
      eyebrow="Local operator timeline"
      accent="mint"
      className="personal-ops personal-ops--reduced-motion personal-ops--high-contrast"
      actions={<span className="module-card__tag">{brief?.quiet_mode ? 'Quiet mode' : 'Quiet mode unavailable'}</span>}
    >
      <p className="personal-ops__summary">{snapshot?.resume.resume.summary ?? 'Reconstructing local context…'}</p>
      <section className="personal-ops__next" aria-labelledby="personal-ops-next-action">
        <h3 id="personal-ops-next-action">Next action</h3>
        <p>{nextAction?.content || 'No explicit next action is scheduled.'}</p>
      </section>
      <div className="personal-ops__capture">
        <label htmlFor="personal-ops-capture">Rapid capture</label>
        <textarea
          id="personal-ops-capture"
          aria-label="Rapid capture"
          value={capture}
          disabled={busy || !ops}
          placeholder="Capture a thought; classify it later"
          onChange={(event) => setCapture(event.target.value)}
          onKeyDown={onCaptureKeyDown}
        />
        <button type="button" disabled={busy || !ops || capture.trim().length === 0} onClick={() => void submitCapture()}>
          Save capture
        </button>
        <small>Enter saves. Shift+Enter adds a line. No category is required.</small>
      </div>

      <div className="personal-ops__status" role="status" aria-live="polite">{status}</div>
      {error ? <div className="personal-ops__error" role="alert">{error}</div> : null}

      <div className="personal-ops__grid">
        <section aria-labelledby="personal-ops-today">
          <h3 id="personal-ops-today">Today timeline</h3>
          {today.length === 0 ? <p>No timeline items for today.</p> : (
            <ol className="personal-ops__timeline">
              {today.map((item) => {
                const awaitingAck = Boolean(
                  item.reminder_id
                  && item.reminder_state?.policy.acknowledgement_required
                  && !item.reminder_acknowledged_at,
                )
                return (
                  <li key={item.item_id}>
                    <strong>{item.content || 'Untitled capture'}</strong>
                    <span>{item.kind} · {formatTime(item.scheduled_at ?? item.due_at)}</span>
                    <small>{item.evidence_class.split('_').join(' ')}</small>
                    {awaitingAck ? (
                      <div>
                        <button
                          type="button"
                          disabled={busy}
                          aria-label={`Acknowledge reminder for ${item.content || 'Untitled capture'}`}
                          onClick={() => void acknowledge(item.reminder_id as string)}
                        >
                          Acknowledge
                        </button>
                        <button type="button" disabled={busy} onClick={() => void respondToReminder(item.reminder_id as string, 'deferred')}>Defer</button>
                        <button type="button" disabled={busy} onClick={() => void respondToReminder(item.reminder_id as string, 'dismissed')}>Dismiss</button>
                      </div>
                    ) : null}
                    <label>
                      Schedule
                      <input
                        type="datetime-local"
                        value={scheduleDrafts[item.item_id] ?? ''}
                        onChange={(event) => setScheduleDrafts((current) => ({ ...current, [item.item_id]: event.target.value }))}
                      />
                    </label>
                    <button type="button" disabled={busy || !scheduleDrafts[item.item_id]} onClick={() => void scheduleItem(item.item_id)}>Save schedule</button>
                    <button type="button" disabled={busy} onClick={() => void completeItem(item.item_id)}>Mark complete</button>
                  </li>
                )
              })}
            </ol>
          )}
        </section>

        <section aria-labelledby="personal-ops-inbox">
          <h3 id="personal-ops-inbox">Inbox</h3>
          {inbox.length === 0 ? <p>Inbox clear.</p> : (
            <ul className="personal-ops__list">
              {inbox.map((item) => (
                <li key={item.capture_id}>
                  <span>{item.content || 'Audio capture'}</span>
                  <label>
                    Classify as
                    <select
                      value={inboxKinds[item.capture_id] ?? 'task'}
                      onChange={(event) => setInboxKinds((current) => ({ ...current, [item.capture_id]: event.target.value }))}
                    >
                      <option value="task">Task</option>
                      <option value="reminder">Reminder</option>
                      <option value="note">Note</option>
                      <option value="appointment">Appointment</option>
                      <option value="contact">Contact</option>
                      <option value="health">Health</option>
                    </select>
                  </label>
                  <button type="button" disabled={busy} onClick={() => void classifyInboxItem(item.capture_id)}>Confirm classification</button>
                </li>
              ))}
            </ul>
          )}
          <h3>Waiting</h3>
          {waiting.length === 0 ? <p>No waiting items.</p> : (
            <ul className="personal-ops__list">
              {waiting.map((item) => <li key={item.item_id}>{item.content || 'Untitled capture'}</li>)}
            </ul>
          )}
        </section>
      </div>

      {reviewCandidates.length > 0 ? (
        <section className="personal-ops__review" aria-labelledby="personal-ops-review">
          <h3 id="personal-ops-review">Review-assisted organization</h3>
          <p>Confirm up to 10 suggestions at a time. Nothing is converted automatically.</p>
          <ul className="personal-ops__list">
            {reviewCandidates.map((item) => {
              const selected = selectedReviewIds.has(item.item_id)
              const atLimit = selectedReviewIds.size >= 10 && !selected
              return (
                <li key={`review-${item.item_id}`}>
                  <label>
                    <input
                      type="checkbox"
                      checked={selected}
                      disabled={busy || atLimit}
                      onChange={(event) => setSelectedReviewIds((current) => {
                        const next = new Set(current)
                        if (event.target.checked) next.add(item.item_id)
                        else next.delete(item.item_id)
                        return next
                      })}
                    />
                    {item.content || 'Untitled capture'} → {item.kind}
                  </label>
                  <small>
                    Confidence {item.confidence === null ? 'unavailable' : `${Math.round(item.confidence * 100)}%`}; rationale {item.classification_reason ?? 'unavailable'}
                  </small>
                </li>
              )
            })}
          </ul>
          <button type="button" disabled={busy || selectedReviewIds.size === 0} onClick={() => void confirmSelected()}>
            Confirm selected ({selectedReviewIds.size}/10)
          </button>
        </section>
      ) : null}

      <section className="personal-ops__data-controls" aria-labelledby="personal-ops-data-controls">
        <h3 id="personal-ops-data-controls">Personal data controls</h3>
        <button type="button" disabled={busy} onClick={() => void exportPersonalData()}>
          Export personal data
        </button>
        {deletePending ? (
          <>
            <button type="button" disabled={busy} onClick={() => void deletePersonalData()}>
              Confirm delete personal data
            </button>
            <button type="button" disabled={busy} onClick={() => setDeletePending(false)}>
              Cancel personal data deletion
            </button>
          </>
        ) : (
          <button type="button" disabled={busy} onClick={() => setDeletePending(true)}>
            Delete personal data
          </button>
        )}
        <small>Deletion removes personal application records only. System execution and governance receipts are preserved.</small>
      </section>

      <div className="personal-ops__reminder-summary">
        {brief?.reminders_awaiting_ack === 1
          ? '1 reminder awaiting acknowledgement'
          : `${brief?.reminders_awaiting_ack ?? 0} reminders awaiting acknowledgement`}
      </div>
      <p className="personal-ops__disclosure">{brief?.uncertainty_disclosure ?? 'Brief reconstructed from the local event log.'}</p>
      <p className="personal-ops__placeholder">Calendar sync: not configured; only local scheduling is active. Voice: Hermes text capture is active; HUD microphone capture requires an operator-configured audio adapter.</p>
    </ModuleCard>
  )
}
