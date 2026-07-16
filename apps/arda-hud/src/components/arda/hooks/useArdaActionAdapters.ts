// sigil: REPAIR
import { useEffect } from 'react'
import type { ArdaBundle } from '../../../lib/ardaBundleTypes'
import { approveHumanAugmentation, recordCeoCouncilSession } from '../../../lib/weathertop'
import {
  registerSystemActionAdapter,
  type SystemActionAdapter,
} from '../../../lib/systemActionBus'

function getString(value: unknown, fallback = 'n/a'): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback
}

function getBoolean(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback
}

function commaList(value: unknown): string[] {
  return getString(value, '')
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

export function useArdaActionAdapters(bundle: ArdaBundle | null) {
  useEffect(() => {
    const adapter: SystemActionAdapter = {
      id: 'arda-human-augmentation',
      presets: ['auto', 'local_cli'],
      canHandle: async (action) => action === 'approve_human_augmentation' || action === 'record_ceo_council_session',
      execute: async (action, context) => {
        const rootPath = bundle?.rootPath
        if (!rootPath) {
          return { ok: false, provider: 'arda-human-augmentation', message: 'Core state bundle not loaded yet' }
        }
        const payload = context.payload ?? {}
        if (action === 'record_ceo_council_session') {
          const requestedMemoryLanes = commaList(payload.memory_lanes)
          const rawMemoryWrites = getString(payload.memory_writes, '')
            .split('\n')
            .map((value) => value.trim())
            .filter(Boolean)
          const memoryLanes = [...requestedMemoryLanes]
          const memoryWrites = rawMemoryWrites.map((value, index) => {
            const separatorIndex = value.indexOf(':')
            if (separatorIndex > 0) {
              const lane = value.slice(0, separatorIndex).trim()
              const content = value.slice(separatorIndex + 1).trim()
              if (lane && content) {
                if (!memoryLanes[index]) {
                  memoryLanes[index] = lane
                }
                return content
              }
            }
            return value
          })
          const result = await recordCeoCouncilSession({
            ardaRoot: rootPath,
            objective: getString(payload.objective, ''),
            loopClass: getString(payload.loop_class, 'lightweight'),
            decisionClass: getString(payload.decision_class, 'routine_maintenance'),
            participants: commaList(payload.participants),
            proposals: commaList(payload.proposals),
            objections: commaList(payload.objections),
            validatorsInvoked: commaList(payload.validators),
            memoryLanes,
            memoryWrites,
            synthesis: getString(payload.synthesis, '') || null,
            triadRequired: getBoolean(payload.triad_required, false),
            humanEscalated: getBoolean(payload.human_escalated, false),
            promotedPrivateMemory: getBoolean(payload.promoted_private_memory, false),
            ingress: 'discord',
            ctoMode: 'hybrid',
            outcomeStatus: 'recorded',
          })
          return {
            ok: result.success,
            provider: 'arda-human-augmentation',
            message: result.success ? 'CEO council session recorded' : (result.error ?? 'CEO council session failed'),
            data: result.content ?? result.error ?? null,
          }
        }

        const result = await approveHumanAugmentation({
          ardaRoot: rootPath,
          decisionClass: getString(payload.decision_class, ''),
          commandSignature: typeof payload.command_signature === 'string' ? payload.command_signature : null,
          approvers: commaList(payload.approvers),
          evidence: commaList(payload.evidence),
          note: getString(payload.note, '') || null,
          status: getString(payload.status, 'approved'),
        })
        return {
          ok: result.success,
          provider: 'arda-human-augmentation',
          message: result.success ? 'Human augmentation approval recorded' : (result.error ?? 'approval failed'),
          data: result.content ?? result.error ?? null,
        }
      },
    }
    return registerSystemActionAdapter(adapter)
  }, [bundle])
}
