// sigil: REPAIR
// Queue health projection from queue.jsonl and related summary files.

import { fetchInventoryTree, readFile } from './weathertop'
import type { JsonRecord } from './ardaSource'
import { parseJsonOrNull } from './jsonParse'

export interface QueueHealthProjection {
  countsByStatus: Record<string, number>
  totalRecords: number
  recentActivity: Array<{
    id: string
    status: string
    updatedAt: string
    owner: string
    title: string
    laneSubclass?: string
  }>
  loadedAt: string
}

const DEFAULT_PROJECTION: QueueHealthProjection = {
  countsByStatus: {},
  totalRecords: 0,
  recentActivity: [],
  loadedAt: new Date().toISOString(),
}

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as JsonRecord
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function getString(value: unknown, fallback = ''): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback
}

function getNumber(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

export async function projectQueueHealth(
  rootPath: string,
  queuePath = 'core/projects/tasks/queue.jsonl',
  summaryPath = 'core/state/queue_summary.json',
): Promise<QueueHealthProjection> {
  const queueResult = await readFile(`${rootPath}/${queuePath}`)
  if (!queueResult.success || !queueResult.content) {
    return { ...DEFAULT_PROJECTION, loadedAt: new Date().toISOString() }
  }

  const queueRecords = queueResult.content
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => asRecord(parseJsonOrNull<unknown>(line)))
    .filter((entry): entry is JsonRecord => entry !== null)

  const countsByStatus: Record<string, number> = {}
  for (const entry of queueRecords) {
    const status = getString(asRecord(entry.status)?.status ?? entry.status, 'unknown')
    countsByStatus[status] = (countsByStatus[status] ?? 0) + 1
  }

  const recentActivity = queueRecords
    .map((entry) => ({
      id: getString(entry.id ?? entry.task_id, 'unknown'),
      status: getString(asRecord(entry.status)?.status ?? entry.status, 'unknown'),
      updatedAt: getString(entry.updated_at_utc ?? entry.ts_utc, ''),
      owner: getString(entry.owner ?? 'unassigned'),
      title: getString(entry.title ?? entry.summary ?? entry.task_id, 'Untitled task'),
      laneSubclass: getString(entry.lane_subclass, undefined),
    }))
    .filter((entry) => entry.updatedAt)
    .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt))
    .slice(0, 12)

  const summary = await projectQueueSummary(rootPath, summaryPath)
  const countsByStatusWithSummary = Object.keys(summary.countsByStatus).length > 0
    ? summary.countsByStatus
    : countsByStatus

  return {
    countsByStatus: countsByStatusWithSummary,
    totalRecords: getNumber(summary.totalRecords, getNumber(queueRecords.length, 0)),
    recentActivity,
    loadedAt: summary.loadedAt,
  }
}

export async function projectQueueSummary(
  rootPath: string,
  summaryPath = 'core/state/queue_summary.json',
): Promise<QueueHealthProjection> {
  const summaryResult = await readFile(`${rootPath}/${summaryPath}`)
  if (!summaryResult.success || !summaryResult.content) {
    return { ...DEFAULT_PROJECTION, loadedAt: new Date().toISOString() }
  }

  const summary = asRecord(parseJsonOrNull<unknown>(summaryResult.content))
  if (!summary) {
    return { ...DEFAULT_PROJECTION, loadedAt: new Date().toISOString() }
  }

  const projectTasks = asRecord(summary.project_tasks)
  const countsByStatusRaw = asRecord(projectTasks?.counts_by_status) ?? {}
  const countsByStatus = Object.fromEntries(
    Object.entries(countsByStatusRaw)
      .map(([status, value]) => [getString(status), getNumber(value, 0)])
      .filter(([, count]) => Number(count) > 0),
  )

  const recentTasks = asArray(projectTasks?.recent)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)

  const recentActivity = recentTasks
    .slice(-12)
    .reverse()
    .map((entry, index) => ({
      id: getString(entry.id ?? entry.task_id, `summary-${index + 1}`),
      status: getString(asRecord(entry.status)?.status ?? entry.status, 'unknown'),
      updatedAt: getString(entry.updated_at_utc ?? entry.ts_utc, ''),
      owner: getString(entry.owner, 'unassigned'),
      title: getString(entry.title ?? entry.summary ?? entry.task_id, 'Untitled task'),
      laneSubclass: getString(entry.lane_subclass, undefined),
    }))
    .filter((entry) => entry.updatedAt)

  return {
    countsByStatus,
    totalRecords: getNumber(projectTasks?.total_count, recentTasks.length),
    recentActivity,
    loadedAt: getString(summary.generated_at_utc, new Date().toISOString()),
  }
}

export async function readQueueRecords(rootPath: string, queuePath = 'core/projects/tasks/queue.jsonl') {
  const queueResult = await readFile(`${rootPath}/${queuePath}`)
  if (!queueResult.success || !queueResult.content) {
    return []
  }

  return queueResult.content
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => asRecord(parseJsonOrNull<unknown>(line)))
    .filter((entry): entry is JsonRecord => entry !== null)
}
