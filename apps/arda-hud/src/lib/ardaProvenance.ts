export type ArdaFreshnessState =
  | 'fresh'
  | 'stale'
  | 'missing'
  | 'derived'
  | 'blocked'
  | 'unknown';

export interface ArdaSourceProvenance {
  domainId: string;
  label: string;
  sourcePaths: string[];
  generatedAtUtc: string | null;
  observedAtUtc: string | null;
  state: ArdaFreshnessState;
  sourceKind: 'snapshot' | 'live' | 'derived' | 'config' | 'manual';
  derivedFrom?: string[];
  safeRefreshCommand?: string;
  lastRefreshResult?: {
    success: boolean;
    message?: string;
    timestampUtc: string;
  };
  notes?: string;
}

export type ArdaRefreshAffordanceSafety = 'read_only' | 'projection_refresh_only' | 'approval_required' | 'not_registered';

export interface ArdaRefreshAffordance {
  id: string;
  label: string;
  safety: ArdaRefreshAffordanceSafety;
  command?: string;
  systemActionId?: string;
  executionMode: 'display_only';
  operatorInstruction: string;
  notes: string;
}

export function normalizeTimestamp(timestamp: string | null | undefined): string | null {
  if (!timestamp) return null;
  try {
    return new Date(timestamp).toISOString();
  } catch {
    return null;
  }
}

export function calculateAgeSeconds(baseTime: Date, timestamp: string | null): number | null {
  if (!timestamp) return null;
  const ts = new Date(timestamp);
  if (isNaN(ts.getTime())) return null;
  return Math.max(0, (baseTime.getTime() - ts.getTime()) / 1000);
}

export function classifyFreshness(
  generatedAt: string | null,
  observedAt: string | null,
  sourceStatus: 'present' | 'missing' | 'error',
  maxAgeSeconds: number = 300
): ArdaFreshnessState {
  const now = new Date();
  const generatedAge = calculateAgeSeconds(now, normalizeTimestamp(generatedAt));
  const observedAge = calculateAgeSeconds(now, normalizeTimestamp(observedAt));

  // If source is missing
  if (sourceStatus === 'missing') return 'missing';

  // If source has error
  if (sourceStatus === 'error') return 'unknown';

  // If we don't know when it was created, unknown
  if (!generatedAt && !observedAt) return 'unknown';

  // Use observed time if no generated time
  const age = generatedAge !== null ? generatedAge : observedAge || Infinity;

  if (age <= maxAgeSeconds) return 'fresh';
  return 'stale';
}

export function getRefreshAffordanceForSourcePath(sourcePath: string): ArdaRefreshAffordance {
  const normalizedPath = sourcePath.toLowerCase();

  if (
    normalizedPath.endsWith('core/state/operator_runtime_status.json') ||
    normalizedPath.endsWith('core/state/manwe_router.json') ||
    normalizedPath.includes('fleet')
  ) {
    return {
      id: 'operator-runtime-status-refresh',
      label: 'Refresh operator runtime projection',
      safety: 'projection_refresh_only',
      command: 'cargo run -p arda-cli -- utility operator-runtime-status',
      systemActionId: normalizedPath.endsWith('core/state/manwe_router.json') ? 'charon.refresh_provider_intelligence' : undefined,
      executionMode: 'display_only',
      operatorInstruction: 'Run from the repository root when operator/runtime or routing projections are stale.',
      notes: 'ARDA displays this command as guidance only; it does not execute projection refreshes from provenance UI.',
    };
  }

  if (
    normalizedPath.endsWith('data/athena/policy_readiness.jsonl') ||
    normalizedPath.endsWith('core/state/athena_runtime.json')
  ) {
    return {
      id: 'athena-policy-readiness-refresh',
      label: 'Refresh ATHENA policy readiness',
      safety: 'projection_refresh_only',
      command: 'cargo run -p arda-cli -- athena policy-readiness --limit 25',
      systemActionId: 'arda.athena_refresh_digest',
      executionMode: 'display_only',
      operatorInstruction: 'Run from the repository root when ATHENA policy readiness or runtime projection data is stale.',
      notes: 'Policy promotion remains separate and approval-gated; this affordance is limited to readiness projection guidance.',
    };
  }

  if (
    normalizedPath.endsWith('data/athena/digest.jsonl') ||
    normalizedPath.endsWith('data/athena/deep_graph.jsonl')
  ) {
    return {
      id: 'athena-digest-inspect',
      label: 'Inspect ATHENA digest projection',
      safety: 'read_only',
      command: 'cargo run -p arda-cli -- athena digest --limit 25',
      systemActionId: 'arda.athena_refresh_digest',
      executionMode: 'display_only',
      operatorInstruction: 'Run from the repository root to inspect current ATHENA digest evidence before deciding whether a refresh action is needed.',
      notes: 'This command reads digest state. Mutating knowledge ingestion or promotion requires explicit operator action elsewhere.',
    };
  }

  if (
    normalizedPath.endsWith('core/projects/tasks/queue.jsonl') ||
    normalizedPath.endsWith('core/state/queue_summary.json') ||
    normalizedPath.includes('hades/lifecycle_review_queue')
  ) {
    return {
      id: 'queue-refresh-approval-required',
      label: 'Queue refresh requires an explicit workflow',
      safety: 'approval_required',
      systemActionId: 'arda.queue_preview_cleanup',
      executionMode: 'display_only',
      operatorInstruction: 'Use the queue or HADES workstation action surface so queue cleanup or task writes remain reviewable.',
      notes: 'Queue operations can change operator work state; provenance UI must not offer one-click execution.',
    };
  }

  return {
    id: 'no-safe-refresh-registered',
    label: 'No safe refresh registered',
    safety: 'not_registered',
    executionMode: 'display_only',
    operatorInstruction: 'Inspect the source path and choose the owning workstation or CLI workflow manually.',
    notes: 'No read-only or projection-refresh-only affordance is registered for this source path.',
  };
}

export function getRefreshAffordance(record: ArdaSourceProvenance): ArdaRefreshAffordance {
  if (record.safeRefreshCommand) {
    return {
      id: `${record.domainId}:safe-refresh-command`,
      label: 'Safe refresh command',
      safety: 'projection_refresh_only',
      command: record.safeRefreshCommand,
      executionMode: 'display_only',
      operatorInstruction: 'Run from the repository root when this projection is stale.',
      notes: 'ARDA displays this command as guidance only and does not execute it from provenance UI.',
    };
  }

  const primaryPath = record.sourcePaths[0];
  if (!primaryPath) {
    return {
      id: `${record.domainId}:no-source-path`,
      label: 'No refresh path available',
      safety: 'not_registered',
      executionMode: 'display_only',
      operatorInstruction: 'No source path is available for refresh guidance.',
      notes: 'The source record must include a source path before ARDA can select a refresh affordance.',
    };
  }

  return getRefreshAffordanceForSourcePath(primaryPath);
}

export function getSafeRefreshCommand(sourcePath: string): string | undefined {
  const affordance = getRefreshAffordanceForSourcePath(sourcePath);
  if (affordance.safety === 'read_only' || affordance.safety === 'projection_refresh_only') {
    return affordance.command;
  }

  return undefined;
}

export function getOperatorLabel(domainId: string): string {
  const labels: Record<string, string> = {
    'source-map': 'Source Map',
    'world': 'World Runtime',
    'manwe-routing': 'Manwe Routing',
    'operator-runtime': 'Operator Runtime',
    'athena-runtime': 'ATHENA Runtime',
    'athena-digest': 'ATHENA Digest',
    'athena-deep-graph': 'ATHENA Deep Graph',
    'athena-policy-readiness': 'ATHENA Policy Readiness',
    'queue-plans': 'Queue & Plans',
    'review-gates': 'Review Gates',
    'fleet-drift': 'Fleet Drift',
    'storage-projection': 'Storage Projection',
    'output-projection': 'Output Projection',
    'package-projection': 'Package Projection',
    'governance': 'Governance',
    'autonomy': 'Autonomy',
    'human': 'Human Input',
    'business': 'Business Logic',
    'personal': 'Personal Data',
  };
  return labels[domainId] || domainId.replace(/-/g, ' ').replace(/\b\w/g, l => l.toUpperCase());
}
