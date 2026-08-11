export type OperatorWindowId = 'main' | string;

export type HermesRuntimeHealth = {
  readonly schemaVersion: 'arda.system-health.hermes.v1';
  readonly state: 'healthy' | 'degraded' | 'starting' | 'unavailable' | 'failed';
  readonly sourceRevision: string;
  readonly sourceTimeUtc: string;
  readonly runtimeAvailable: boolean;
  readonly runtimeIdentity: string | null;
  readonly runtimeLaunched: boolean;
  readonly runtimeReady: boolean;
  readonly url: string;
  readonly port: number;
  readonly probes: Readonly<{
    readonly port: boolean;
    readonly identity: boolean;
  }>;
  readonly failure: string | null;
  readonly recoveryAction: string | null;
};

export type LaunchHermesRuntimeResult = {
  readonly launched: boolean;
  readonly ready: boolean;
  readonly url: string | null;
  readonly port: number | null;
  readonly runtimeIdentity?: string | null;
  readonly state?: string;
  readonly source?: string | null;
  readonly failure?: string | null;
};

class OperatorStore {
  #health: HermesRuntimeHealth = {
    schemaVersion: 'arda.system-health.hermes.v1',
    state: 'unavailable',
    sourceRevision: 'unobserved',
    sourceTimeUtc: '',
    runtimeAvailable: false,
    runtimeIdentity: null,
    runtimeLaunched: false,
    runtimeReady: false,
    url: 'http://127.0.0.1:9119',
    port: 9119,
    probes: {
      port: false,
      identity: false,
    },
    failure: null,
    recoveryAction: null,
  };

  readonly maxSpots: number;

  constructor(options: { readonly maxSpots?: number } = {}) {
    this.maxSpots = Math.max(1, options.maxSpots ?? 24);
  }

  get current(): HermesRuntimeHealth {
    return this.#health;
  }

  patch(
    partial: Partial<HermesRuntimeHealth>,
  ): HermesRuntimeHealth {
    if (
      partial.sourceTimeUtc
      && this.#health.sourceTimeUtc
      && partial.sourceTimeUtc < this.#health.sourceTimeUtc
    ) {
      return this.#health;
    }
    this.#health = {
      ...this.#health,
      ...partial,
      probes: partial.probes ?? this.#health.probes,
    };
    return this.#health;
  }

  reset(): void {
    this.#health = {
      schemaVersion: 'arda.system-health.hermes.v1',
      state: 'unavailable',
      sourceRevision: 'unobserved',
      sourceTimeUtc: '',
      runtimeAvailable: false,
      runtimeIdentity: null,
      runtimeLaunched: false,
      runtimeReady: false,
      url: 'http://127.0.0.1:9119',
      port: 9119,
      probes: {
        port: false,
        identity: false,
      },
      failure: null,
      recoveryAction: null,
    };
  }
}

export const operatorStore = new OperatorStore({ maxSpots: 24 });

export default operatorStore;
