import { invoke } from '@tauri-apps/api/core'

export type RegistryGate = 'pass' | 'warn' | 'fail'

export interface RegistryTrack {
  track_id: string
  title: string
  owner: string
  status: string
  source_modules: string[]
  receipt_stores: string[]
}

export interface RegistryStatusPayload {
  loaded: boolean
  schema_version: string
  authority: string
  track_count: number
  gate_status: RegistryGate
  tracks: RegistryTrack[]
  checked_at_utc: string
  error: string | null
}

export interface ReadinessProjection {
  gate_status: RegistryGate
  mode: string
  mutation_policy: string
  summary: Record<string, number>
  pass: string[]
  warn: string[]
}

export interface ServiceAction {
  action_id: string
  action_type: string
  title: string
  command_hint: string
  target_path: string | null
  requires_human_gate: boolean
  description: string
  risk: string
}

export interface ServicePlan {
  contract: string
  generated_at_utc: string
  profile: string
  machine_role: string
  gate_status: string
  approval_contract_required: string
  actions: ServiceAction[]
}

export interface OnboardingSnapshot {
  readiness: ReadinessProjection | null
  servicePlan: ServicePlan | null
}

type RootArgs = { root?: string }

async function invokeCommand<T>(command: string, args: RootArgs): Promise<T> {
  if (typeof window !== 'undefined' && !('__TAURI__' in window)) {
    console.warn(`${command}: __TAURI__ missing in window`, { args })
  }
  try {
    return await invoke<T>(command, args)
  } catch (err) {
    console.error(`${command} failed`, args, err)
    throw err
  }
}

export function invokeRegistryStatus(args: RootArgs): Promise<RegistryStatusPayload> {
  return invokeCommand('registry_status', args)
}

export function invokeReadinessStatus(args: RootArgs): Promise<ReadinessProjection | null> {
  return invokeCommand('readiness_status', args)
}

export function invokeServicePlanStatus(args: RootArgs): Promise<ServicePlan | null> {
  return invokeCommand('service_plan_status', args)
}

export async function invokeOnboardingSnapshot(args: RootArgs): Promise<OnboardingSnapshot> {
  const [readiness, servicePlan] = await Promise.all([
    invokeReadinessStatus(args),
    invokeServicePlanStatus(args),
  ])
  return { readiness, servicePlan }
}
