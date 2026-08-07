import { useCallback, useEffect, useMemo, useState, type KeyboardEvent } from 'react'
import { createPersonalOpsClient, type PersonalOpsClient, type PersonalOpsSnapshot } from '../../../lib/personalOps'
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
  operatorId = 'operator-0',
}: PersonalOperationsModuleProps) {
  const defaultClient = useMemo(() => createPersonalOpsClient(operatorId), [operatorId])
  const ops = client ?? defaultClient
  const [snapshot, setSnapshot] = useState<PersonalOpsSnapshot | null>(null)
  const [capture, setCapture] = useState('')
  const [busy, setBusy] = useState(false)
  const [selectedReviewIds, setSelectedReviewIds] = useState<Set<string>>(new Set())
  const [deletePending, setDeletePending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [status, setStatus] = useState('Loading personal operations')

  const refresh = useCallback(async () => {
    const next = await ops.loadSnapshot()
    setSnapshot(next)
    setStatus('Personal operations loaded')
    setError(null)
  }, [ops])

  useEffect(() => {
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
    if (!text || busy) return
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
    if (busy) return
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
    if (selected.length === 0 || busy) return
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
    if (busy) return
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
    if (busy) return
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
          disabled={busy}
          placeholder="Capture a thought; classify it later"
          onChange={(event) => setCapture(event.target.value)}
          onKeyDown={onCaptureKeyDown}
        />
        <button type="button" disabled={busy || capture.trim().length === 0} onClick={() => void submitCapture()}>
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
                      <button
                        type="button"
                        disabled={busy}
                        aria-label={`Acknowledge reminder for ${item.content || 'Untitled capture'}`}
                        onClick={() => void acknowledge(item.reminder_id as string)}
                        onKeyDown={(event) => {
                          if (event.key === 'Enter' || event.key === ' ') {
                            event.preventDefault()
                            void acknowledge(item.reminder_id as string)
                          }
                        }}
                      >
                        Acknowledge
                      </button>
                    ) : null}
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
              {inbox.map((item) => <li key={item.capture_id}>{item.content || 'Audio capture'}</li>)}
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
      <p className="personal-ops__placeholder">Calendar automation and voice capture remain a supervised-adapter placeholder until configured.</p>
    </ModuleCard>
  )
}
