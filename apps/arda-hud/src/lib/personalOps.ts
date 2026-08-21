export interface PersonalOpsInboxItem {
  capture_id: string
  operator_id: string
  content: string
  audio_reference: string | null
  occurred_at: string
}

export interface PersonalOpsReminderState {
  delivery_state: string
  attempt_count: number
  last_acknowledged_at: string | null
  policy: {
    interruption: string
    quiet_window: unknown
    max_attempts: number
    minimum_interval_minutes: number
    acknowledgement_required: boolean
  }
  non_clinical_disclosure: string
}

export interface PersonalOpsItem {
  item_id: string
  kind: string
  operator_id: string
  content: string
  evidence_class: string
  confidence: number | null
  classification_reason: string
  scheduled_at: string | null
  due_at: string | null
  completed_at: string | null
  reminder_id?: string | null
  reminder_state: PersonalOpsReminderState | null
  reminder_attempts: number
  reminder_acknowledged_at: string | null
  current_state: string
}

export interface PersonalAdapterStatus {
  state: 'configured' | 'unconfigured' | 'unavailable'
  adapter: string | null
  detail: string
}

export interface PersonalReminderTransportStatus extends PersonalAdapterStatus {
  quiet_window: { start: string; end: string; timezone: string } | null
  max_attempts: number
  minimum_interval_minutes: number
  acknowledgement_required: boolean
}

export interface NextActionCandidate {
  id: string
  title: string
  source_kind: 'queue' | 'personal_operations' | 'workbench' | 'research'
  source_ref: string
  reason: string
  freshness: 'fresh' | 'stale' | 'unknown'
  authority_state: 'ready' | 'review_required' | 'blocked' | 'advisory'
  next_operator_action: string
  priority: number
  operator_authored: boolean
  terminal: boolean
  future_gated: boolean
  inferred_without_review: boolean
}

export interface NextActionProjection {
  schema_version: 'arda.next-action.v1'
  generated_at: string
  status: 'ready' | 'blocked' | 'empty'
  selected: NextActionCandidate | null
  reason: string
  excluded: {
    stale: number
    terminal: number
    future_gated: number
    inferred_without_review: number
  }
}

export interface PersonalOpsSnapshot {
  capabilities: {
    schema_version: 'arda.personal-capabilities.v1'
    calendar: PersonalAdapterStatus
    voice: PersonalAdapterStatus
    reminders: PersonalReminderTransportStatus
  }
  nextAction: NextActionProjection
  inbox: {
    schema_version: string
    inbox: PersonalOpsInboxItem[]
  }
  resume: {
    schema_version: string
    resume: {
      summary: string
      active_count: number
      inbox_count: number
      today_count: number
      waiting_count: number
      generated_at: string
    }
  }
  todayBrief: {
    schema_version: string
    brief: {
      generated_at: string
      today: PersonalOpsItem[]
      waiting: PersonalOpsItem[]
      reminders_awaiting_ack: number
      quiet_mode: boolean
      uncertainty_disclosure: string
    }
  }
}

export interface PersonalDataExport {
  schema_version: 'arda.personal-data-export.v1'
  generated_at: string
  operator_id: string
  events: unknown[]
}

export interface PersonalOpsClient {
  loadSnapshot(): Promise<PersonalOpsSnapshot>
  createCapture(text: string): Promise<{ event_id: string; capture_id: string }>
  confirmClassification(itemId: string, kind: string): Promise<{ event_id: string }>
  scheduleItem(itemId: string, scheduledAt: string): Promise<{ event_id: string }>
  completeItem(itemId: string): Promise<{ event_id: string }>
  acknowledgeReminder(reminderId: string): Promise<{ event_id: string }>
  respondToReminder(reminderId: string, state: 'acknowledged' | 'deferred' | 'dismissed'): Promise<{ event_id: string }>
  exportPersonalData(): Promise<PersonalDataExport>
  deletePersonalData(): Promise<{ receipt_id: string; deleted_events: number; system_receipts_modified: false }>
}

interface HarnessStatus {
  operator_id?: unknown
}

export function buildPersonalOpsUrl(
  path: string,
  base = import.meta.env.VITE_ARDA_HARNESS_URL ?? 'http://127.0.0.1:7878',
): string {
  return `${base.replace(/\/$/, '')}/${path.replace(/^\//, '')}`
}

function idempotencyKey(action: string): string {
  const suffix = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`
  return `arda-hud-${action}-${suffix}`
}

async function readJson<T>(response: Response): Promise<T> {
  if (response.ok) return response.json() as Promise<T>
  let detail = `${response.status} ${response.statusText}`.trim()
  try {
    const body = await response.json() as { error?: unknown }
    if (typeof body.error === 'string') detail = body.error
  } catch {
    // Preserve the status when an upstream proxy returns a non-JSON body.
  }
  throw new Error(`Personal operations request failed: ${detail}`)
}

export async function loadConfiguredOperatorId(baseUrl?: string): Promise<string> {
  const status = await fetch(buildPersonalOpsUrl('/v1/status', baseUrl)).then(readJson<HarnessStatus>)
  if (typeof status.operator_id !== 'string' || status.operator_id.trim().length === 0) {
    throw new Error('Personal operations request failed: harness did not publish a configured operator identity')
  }
  return status.operator_id.trim()
}

export function createPersonalOpsClient(
  operatorId: string,
  baseUrl?: string,
): PersonalOpsClient {
  const configuredOperatorId = operatorId.trim()
  if (!configuredOperatorId) throw new Error('Personal operations require a configured operator identity')
  const url = (path: string) => buildPersonalOpsUrl(path, baseUrl)
  const mutationHeaders = (action: string) => ({
    'content-type': 'application/json',
    'x-arda-operator-id': configuredOperatorId,
    'idempotency-key': idempotencyKey(action),
  })
  const get = <T>(path: string) => fetch(url(path), {
    headers: { 'x-arda-operator-id': configuredOperatorId },
  }).then(readJson<T>)

  return {
    async loadSnapshot() {
      const [capabilities, nextAction, inbox, resume, todayBrief] = await Promise.all([
        get<PersonalOpsSnapshot['capabilities']>('/v1/personal/capabilities'),
        get<NextActionProjection>('/v1/next-action'),
        get<PersonalOpsSnapshot['inbox']>('/v1/personal/inbox'),
        get<PersonalOpsSnapshot['resume']>('/v1/personal/resume'),
        get<PersonalOpsSnapshot['todayBrief']>('/v1/personal/briefs/today'),
      ])
      return { capabilities, nextAction, inbox, resume, todayBrief }
    },
    createCapture(text) {
      return fetch(url('/v1/personal/captures'), {
        method: 'POST',
        headers: mutationHeaders('capture'),
        body: JSON.stringify({ operator_id: configuredOperatorId, text }),
      }).then(readJson<{ event_id: string; capture_id: string }>)
    },
    confirmClassification(itemId, kind) {
      return fetch(url(`/v1/personal/items/${encodeURIComponent(itemId)}/classify`), {
        method: 'POST',
        headers: mutationHeaders('classify'),
        body: JSON.stringify({
          operator_id: configuredOperatorId,
          item_id: itemId,
          kind,
          evidence_class: 'operator_authored',
          rationale: 'Confirmed in bounded HUD review',
        }),
      }).then(readJson<{ event_id: string }>)
    },
    scheduleItem(itemId, scheduledAt) {
      return fetch(url(`/v1/personal/items/${encodeURIComponent(itemId)}/schedule`), {
        method: 'POST',
        headers: mutationHeaders('schedule'),
        body: JSON.stringify({
          operator_id: configuredOperatorId,
          scheduled_at: scheduledAt,
          due_at: null,
        }),
      }).then(readJson<{ event_id: string }>)
    },
    completeItem(itemId) {
      return fetch(url(`/v1/personal/items/${encodeURIComponent(itemId)}/complete`), {
        method: 'POST',
        headers: mutationHeaders('complete'),
        body: JSON.stringify({ operator_id: configuredOperatorId }),
      }).then(readJson<{ event_id: string }>)
    },
    acknowledgeReminder(reminderId) {
      return fetch(url(`/v1/personal/reminders/${encodeURIComponent(reminderId)}/acknowledge`), {
        method: 'POST',
        headers: mutationHeaders('acknowledge'),
        body: JSON.stringify({ operator_id: configuredOperatorId, state: 'acknowledged' }),
      }).then(readJson<{ event_id: string }>)
    },
    respondToReminder(reminderId, state) {
      return fetch(url(`/v1/personal/reminders/${encodeURIComponent(reminderId)}/acknowledge`), {
        method: 'POST',
        headers: mutationHeaders('acknowledge'),
        body: JSON.stringify({ operator_id: configuredOperatorId, state }),
      }).then(readJson<{ event_id: string }>)
    },
    async exportPersonalData() {
      return fetch(url('/v1/personal/data/export'), {
        headers: { 'x-arda-operator-id': configuredOperatorId },
      }).then(readJson<PersonalDataExport>)
    },
    deletePersonalData() {
      return fetch(url('/v1/personal/data'), {
        method: 'DELETE',
        headers: mutationHeaders('delete-personal-data'),
        body: JSON.stringify({ operator_id: configuredOperatorId }),
      }).then(readJson<{ receipt_id: string; deleted_events: number; system_receipts_modified: false }>)
    },
  }
}
