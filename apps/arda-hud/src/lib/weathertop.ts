// sigil: REPAIR
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { envEndpointUrl, envString } from './endpointConfig'

const WEATHERTOP_SCHEME = envString(import.meta.env.VITE_WEATHERTOP_SCHEME, 'http')
const WEATHERTOP_HOST = envString(import.meta.env.VITE_WEATHERTOP_HOST, '127.0.0.1')
const WEATHERTOP_PORT = envString(import.meta.env.VITE_WEATHERTOP_PORT, '8000')
const WEATHERTOP_BASE_URL = envEndpointUrl({ url: import.meta.env.VITE_WEATHERTOP_URL, scheme: WEATHERTOP_SCHEME, host: WEATHERTOP_HOST, port: WEATHERTOP_PORT })
const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export function getWeathertopBaseUrl(): string {
  return WEATHERTOP_BASE_URL
}

export async function postAction(action: {
  type: string
  target: string
  payload?: Record<string, unknown>
}): Promise<boolean> {
  try {
    const res = await fetch(`${WEATHERTOP_BASE_URL}/action`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(action)
    })
    return res.ok
  } catch {
    return false
  }
}

// ============ Tauri IPC Types ============

export interface FileReadResult {
  success: boolean
  content: string | null
  error: string | null
  path: string
}

export interface SourcePathResult {
  ok: boolean
  sourcePath: string
  resolvedPath: string
  message: string
}

export interface SourceImagePreviewResult {
  ok: boolean
  sourcePath: string
  resolvedPath: string
  mimeType: string
  sizeBytes: number
  dataUrl: string
}

export interface SourceVideoPreviewResult {
  ok: boolean
  sourcePath: string
  resolvedPath: string
  mimeType: string
  sizeBytes: number
  dataUrl: string
}

export interface SourcePdfPreviewResult {
  ok: boolean
  sourcePath: string
  resolvedPath: string
  mimeType: string
  sizeBytes: number
  dataUrl: string
}

export interface HumanAugmentationApprovalRequest {
  ardaRoot: string
  decisionClass: string
  commandSignature?: string | null
  approvers?: string[]
  evidence?: string[]
  expiresAtUtc?: string | null
  note?: string | null
  status?: string | null
}

export interface CeoCouncilSessionRequest {
  ardaRoot: string
  objective: string
  ceoIdentity?: string | null
  ctoIdentity?: string | null
  ctoMode?: string | null
  ingress?: string | null
  channelRef?: string | null
  loopClass?: string | null
  decisionClass?: string | null
  triadRequired?: boolean
  participants: string[]
  proposals: string[]
  objections: string[]
  synthesis?: string | null
  outcomeStatus?: string | null
  humanEscalated?: boolean
  validatorsInvoked: string[]
  memoryLanes: string[]
  memoryWrites: string[]
  promotedPrivateMemory?: boolean
}

export interface InventoryTreeNode {
  id: string
  name: string
  path: string
  relative_path: string
  is_dir: boolean
  size_bytes: number | null
  modified_unix: number | null
  children: InventoryTreeNode[]
}

export interface HudPulseEvent {
  ts_unix_ms: number
  status: 'healthy'
  source: 'tauri'
  sequence: number
}

// ============ Tauri IPC Functions ============

export async function getArdaRoot(): Promise<string> {
  if (IS_TAURI) return invoke<string>('get_arda_root')
  const envPath = import.meta.env?.VITE_ARDA_ROOT
  if (envPath) return envPath
  const fallbackRoots = [
    '/var/home/mythos/Arda/core/state',
    '/home/user/Arda/core/state',
    '/opt/Arda/core/state',
    '/var/home/mythos/Arda/core/state',
    '/home/user/Arda/core/state',
    '/opt/arda/core/state',
  ]
  return fallbackRoots[0]
}

export async function getNumenorPath(): Promise<string> {
  return getArdaRoot()
}

export async function readFile(path: string): Promise<FileReadResult> {
  return invoke<FileReadResult>('read_file', { path })
}

export async function fetchInventoryTree(
  ardaRoot: string,
  relativePath = '.',
  maxDepth = 4
): Promise<FileReadResult> {
  return invoke<FileReadResult>('fetch_inventory_tree', { ardaRoot, relativePath, maxDepth })
}

export async function createScopedFolder(
  ardaRoot: string,
  relativePath: string
): Promise<FileReadResult> {
  return invoke<FileReadResult>('create_scoped_folder', { ardaRoot, relativePath })
}

export async function renameScopedPath(
  ardaRoot: string,
  oldRelativePath: string,
  newRelativePath: string
): Promise<FileReadResult> {
  return invoke<FileReadResult>('rename_scoped_path', { ardaRoot, oldRelativePath, newRelativePath })
}

export async function deleteScopedPath(
  ardaRoot: string,
  relativePath: string,
  recursive = false
): Promise<FileReadResult> {
  return invoke<FileReadResult>('delete_scoped_path', { ardaRoot, relativePath, recursive })
}

export async function writeScopedFile(
  ardaRoot: string,
  relativePath: string,
  content: string
): Promise<FileReadResult> {
  return invoke<FileReadResult>('write_scoped_file', { numenorPath: ardaRoot, relativePath, content })
}

export async function openSourcePath(sourcePath: string): Promise<SourcePathResult> {
  return invoke<SourcePathResult>('open_source_path', { sourcePath })
}

export async function readSourceImagePreview(sourcePath: string): Promise<SourceImagePreviewResult> {
  return invoke<SourceImagePreviewResult>('read_source_image_preview', { sourcePath })
}

export async function readSourceVideoPreview(sourcePath: string): Promise<SourceVideoPreviewResult> {
  return invoke<SourceVideoPreviewResult>('read_source_video_preview', { sourcePath })
}

export async function readSourcePdfPreview(sourcePath: string): Promise<SourcePdfPreviewResult> {
  return invoke<SourcePdfPreviewResult>('read_source_pdf_preview', { sourcePath })
}

export async function approveHumanAugmentation(
  request: HumanAugmentationApprovalRequest
): Promise<FileReadResult> {
  return invoke<FileReadResult>('approve_human_augmentation_action', { ...request })
}

export async function recordCeoCouncilSession(
  request: CeoCouncilSessionRequest
): Promise<FileReadResult> {
  return invoke<FileReadResult>('record_ceo_council_session_action', { ...request })
}

export async function startHudPulseStream(intervalMs = 15000): Promise<string> {
  return invoke<string>('start_hud_pulse_stream', { intervalMs })
}

export async function stopHudPulseStream(): Promise<string> {
  return invoke<string>('stop_hud_pulse_stream')
}

export async function listenHudPulse(
  onEvent: (event: HudPulseEvent) => void,
): Promise<UnlistenFn> {
  return listen<HudPulseEvent>('arda://hud-pulse', (event) => {
    onEvent(event.payload)
  })
}
