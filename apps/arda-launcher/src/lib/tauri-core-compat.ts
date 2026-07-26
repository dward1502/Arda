import { invoke } from '@tauri-apps/api/core'

export async function invokeRegistryStatus<T>(args: { root?: string }): Promise<T> {
  if (!(window as any).__TAURI__) {
    console.warn('invokeRegistryStatus: __TAURI__ missing in window', { args })
  }
  try {
    return await invoke<T>('registry_status', args)
  } catch (err) {
    console.error('invokeRegistryStatus failed', args, err)
    throw err
  }
}

export async function invokeServicePlanStatus<T>(args: { root?: string }): Promise<T> {
  try {
    return await invoke<T>('service_plan_status', args)
  } catch (err) {
    console.error('invokeServicePlanStatus failed', args, err)
    throw err
  }
}
