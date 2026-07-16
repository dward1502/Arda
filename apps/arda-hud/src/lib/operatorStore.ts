export type OperatorWindowId = 'main' | string;

export type HermesRuntimeHealth = {
  readonly runtimeAvailable: boolean;
  readonly runtimeIdentity: string | null;
  readonly runtimeLaunched: boolean;
  readonly runtimeReady: boolean;
  readonly runtimeVersion: string | null;
  readonly sessionDirectory: string | null;
  readonly spotsCount: number;
  readonly spotsActive: number;
  readonly url: string | null;
  readonly port: number | null;
  readonly probes: Readonly<{
    readonly port: boolean;
    readonly identity: boolean;
    readonly version: boolean;
    readonly sessionDirectory: boolean;
    readonly atLeastOneSpot: boolean;
  }>;
};

export type LaunchHermesRuntimeResult = {
  readonly launched: boolean;
  readonly ready: boolean;
  readonly url: string | null;
  readonly port: number | null;
  readonly identity: string | null;
  readonly spotCount: number;
  readonly source?: string | null;
  readonly failure?: string | null;
};

class OperatorStore {
  #health: HermesRuntimeHealth = {
    runtimeAvailable: false,
    runtimeIdentity: null,
    runtimeLaunched: false,
    runtimeReady: false,
    runtimeVersion: null,
    sessionDirectory: null,
    spotsCount: 0,
    spotsActive: 0,
    url: null,
    port: null,
    probes: {
      port: false,
      identity: false,
      version: false,
      sessionDirectory: false,
      atLeastOneSpot: false,
    },
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
    this.#health = {
      ...this.#health,
      ...partial,
      probes: partial.probes ?? this.#health.probes,
    };
    return this.#health;
  }

  reset(): void {
    this.#health = {
      runtimeAvailable: false,
      runtimeIdentity: null,
      runtimeLaunched: false,
      runtimeReady: false,
      runtimeVersion: null,
      sessionDirectory: null,
      spotsCount: 0,
      spotsActive: 0,
      url: null,
      port: null,
      probes: {
        port: false,
        identity: false,
        version: false,
        sessionDirectory: false,
        atLeastOneSpot: false,
      },
    };
  }
}

export const operatorStore = new OperatorStore({ maxSpots: 24 });

export default operatorStore;
