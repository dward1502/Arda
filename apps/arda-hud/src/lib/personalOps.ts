import { safeTauriInvoke } from './tauriGuard'

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

export type PersonalOpsLoadState = 'healthy' | 'stale' | 'degraded' | 'unavailable' | 'failed'

export interface PersonalOpsSnapshot {
  schemaVersion: 'arda.hud.personal-ops-projection.v1'
  state: PersonalOpsLoadState
  sourceRevision: string
  sourceTimeUtc: string
  failures: string[]
  recoveryAction: string | null
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
  acknowledgeReminder(reminderId: string): Promise<{ event_id: string }>
  exportPersonalData(): Promise<PersonalDataExport>
  deletePersonalData(): Promise<{ receipt_id: string; deleted_events: number; system_receipts_modified: false }>
}

export function createPersonalOpsClient(): PersonalOpsClient {
  return {
    loadSnapshot: () => safeTauriInvoke<PersonalOpsSnapshot>('get_personal_ops_projection'),
    createCapture: (text) => safeTauriInvoke('create_personal_capture', { intent: { text } }),
    confirmClassification: (itemId, kind) => safeTauriInvoke('confirm_personal_classification', { intent: { itemId, kind } }),
    acknowledgeReminder: (reminderId) => safeTauriInvoke('acknowledge_personal_reminder', { intent: { reminderId } }),
    exportPersonalData: () => safeTauriInvoke<PersonalDataExport>('export_personal_data'),
    deletePersonalData: () => safeTauriInvoke('delete_personal_data', { intent: { confirmation: 'delete-personal-data' } }),
  }
}
