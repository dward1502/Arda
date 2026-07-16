import { 
  ArdaSourceProvenance, 
  classifyFreshness, 
  normalizeTimestamp, 
  calculateAgeSeconds,
  getOperatorLabel,
  getRefreshAffordanceForSourcePath,
  getSafeRefreshCommand,
} from './ardaProvenance';

const now = new Date();
const fiveMinAgo = new Date(now.getTime() - 300000).toISOString();
const tenMinAgo = new Date(now.getTime() - 600000).toISOString();
const twoHoursAgo = new Date(now.getTime() - 7200000).toISOString();

describe('normalizeTimestamp', () => {
  test('normalizes valid ISO string', () => {
    const input = '2026-05-22T10:00:00.000Z';
    const result = normalizeTimestamp(input);
    expect(result).toBe('2026-05-22T10:00:00.000Z');
  });

  test('returns null for invalid string', () => {
    const result = normalizeTimestamp('invalid-date');
    expect(result).toBeNull();
  });

  test('returns null for null/undefined', () => {
    expect(normalizeTimestamp(null)).toBeNull();
    expect(normalizeTimestamp(undefined)).toBeNull();
  });
});

describe('calculateAgeSeconds', () => {
  test('calculates age correctly', () => {
    const age = calculateAgeSeconds(now, fiveMinAgo);
    expect(age).toBeCloseTo(300, 0);
  });

  test('returns null for invalid timestamp', () => {
    const age = calculateAgeSeconds(now, 'invalid-date');
    expect(age).toBeNull();
  });

  test('returns null for null timestamp', () => {
    const age = calculateAgeSeconds(now, null);
    expect(age).toBeNull();
  });
});

describe('classifyFreshness', () => {
  test('returns fresh when generated recently', () => {
    const state = classifyFreshness(fiveMinAgo, null, 'present', 600);
    expect(state).toBe('fresh');
  });

  test('returns stale when generated long ago', () => {
    const state = classifyFreshness(twoHoursAgo, null, 'present', 600);
    expect(state).toBe('stale');
  });

  test('returns missing when source is missing', () => {
    const state = classifyFreshness(null, null, 'missing');
    expect(state).toBe('missing');
  });

  test('returns unknown for no timestamps', () => {
    const state = classifyFreshness(null, null, 'present');
    expect(state).toBe('unknown');
  });

  test('returns unknown for error source', () => {
    const state = classifyFreshness(fiveMinAgo, null, 'error');
    expect(state).toBe('unknown');
  });

  test('uses observed time if generated is missing', () => {
    const state = classifyFreshness(null, fiveMinAgo, 'present', 600);
    expect(state).toBe('fresh');
  });
});

describe('getOperatorLabel', () => {
  test('returns known operator labels', () => {
    expect(getOperatorLabel('source-map')).toBe('Source Map');
    expect(getOperatorLabel('world')).toBe('World Runtime');
  });

  test('formats unknown IDs with capitalized words', () => {
    expect(getOperatorLabel('custom-domain')).toBe('Custom Domain');
  });
});

describe('getSafeRefreshCommand', () => {
  test('returns the read-only operator runtime refresh command for operator and Charon projections', () => {
    expect(getSafeRefreshCommand('core/state/operator_runtime_status.json')).toBe(
      'cargo run -p arda-cli -- utility operator-runtime-status'
    );
    expect(getSafeRefreshCommand('core/state/charon_router.json')).toBe(
      'cargo run -p arda-cli -- utility operator-runtime-status'
    );
  });

  test('returns the read-only ATHENA policy readiness command for ATHENA projections', () => {
    expect(getSafeRefreshCommand('data/athena/policy_readiness.jsonl')).toBe(
      'cargo run -p arda-cli -- athena policy-readiness --limit 25'
    );
    expect(getSafeRefreshCommand('core/state/athena_runtime.json')).toBe(
      'cargo run -p arda-cli -- athena policy-readiness --limit 25'
    );
  });

  test('does not expose mutating or unknown commands as safe refresh affordances', () => {
    expect(getSafeRefreshCommand('data/athena/policy_promote.jsonl')).toBeUndefined();
    expect(getSafeRefreshCommand('core/state/world.json')).toBeUndefined();
  });
});

describe('getRefreshAffordanceForSourcePath', () => {
  test('returns a display-only projection refresh contract for runtime projections', () => {
    expect(getRefreshAffordanceForSourcePath('core/state/charon_router.json')).toMatchObject({
      id: 'operator-runtime-status-refresh',
      safety: 'projection_refresh_only',
      executionMode: 'display_only',
      command: 'cargo run -p arda-cli -- utility operator-runtime-status',
    });
  });

  test('marks queue refreshes as approval-gated instead of directly executable', () => {
    const affordance = getRefreshAffordanceForSourcePath('core/projects/tasks/queue.jsonl');
    expect(affordance).toMatchObject({
      id: 'queue-refresh-approval-required',
      safety: 'approval_required',
      executionMode: 'display_only',
    });
    expect(affordance.command).toBeUndefined();
  });

  test('returns an explicit manual-only contract for unknown sources', () => {
    expect(getRefreshAffordanceForSourcePath('core/state/world.json')).toMatchObject({
      id: 'no-safe-refresh-registered',
      safety: 'not_registered',
      executionMode: 'display_only',
    });
  });
});
