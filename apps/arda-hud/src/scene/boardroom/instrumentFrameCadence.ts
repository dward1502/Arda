export const BOARDROOM_INSTRUMENT_FRAME_INTERVAL_MS = 1000 / 8

export function shouldDrawInstrumentFrame(now: number, lastDrawAt: number): boolean {
  return now - lastDrawAt >= BOARDROOM_INSTRUMENT_FRAME_INTERVAL_MS
}
