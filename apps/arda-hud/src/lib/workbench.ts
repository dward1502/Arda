import { safeTauriInvoke } from './tauriGuard'

export interface WorkbenchPermissionSummary {
  authority: 'deny_by_default' | 'read_only' | 'approval_required'
  networkAllowed: boolean
  filesystemWrite: boolean
  secretEnvNames: string[]
}

export interface ProjectContractValidation {
  schemaVersion: 'arda.project-contract.v1'
  projectId: string
  name: string
  kind: string
  workspaceRoot: string
  runtimeAdapter: string
  commandIds: string[]
  checkIds: string[]
  permissions: WorkbenchPermissionSummary
}

export function validateProjectContract(path: string): Promise<ProjectContractValidation> {
  return safeTauriInvoke<ProjectContractValidation>('validate_project_contract', {
    path: path.trim(),
  })
}

export interface WorkbenchDraftNode {
  id: string
  kind: 'inspect' | 'plan' | 'approval' | 'execute' | 'verify'
  authority: 'read_only' | 'human_approval' | 'execute_with_approval' | 'verify'
  state: 'ready' | 'blocked'
}

export interface WorkbenchRunDraft {
  schemaVersion: 'arda.run-graph.v1'
  objective: string
  projectContractPath: string
  nodes: WorkbenchDraftNode[]
}

export function validateWorkbenchDraftInput(projectContractPath: string, objective: string): string[] {
  const errors: string[] = []
  const path = projectContractPath.trim()
  if (!path.startsWith('/')) {
    errors.push('Project contract must use an absolute path.')
  }
  if (path.split('/').includes('..')) {
    errors.push('Project contract path cannot contain parent traversal.')
  }
  if (!path.endsWith('.json')) {
    errors.push('Project contract path must identify a JSON contract.')
  }
  if (!objective.trim()) {
    errors.push('Objective is required.')
  }
  return errors
}

export function buildWorkbenchRunDraft(
  projectContractPath: string,
  objective: string,
): WorkbenchRunDraft {
  const errors = validateWorkbenchDraftInput(projectContractPath, objective)
  if (errors.length > 0) {
    throw new Error(errors.join(' '))
  }
  return {
    schemaVersion: 'arda.run-graph.v1',
    objective: objective.trim(),
    projectContractPath: projectContractPath.trim(),
    nodes: [
      { id: 'inspect-1', kind: 'inspect', authority: 'read_only', state: 'ready' },
      { id: 'plan-1', kind: 'plan', authority: 'read_only', state: 'blocked' },
      { id: 'approval-1', kind: 'approval', authority: 'human_approval', state: 'blocked' },
      { id: 'execute-1', kind: 'execute', authority: 'execute_with_approval', state: 'blocked' },
      { id: 'verify-1', kind: 'verify', authority: 'verify', state: 'blocked' },
    ],
  }
}
