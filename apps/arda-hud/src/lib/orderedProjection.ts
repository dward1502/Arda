export function sortedEntries<T extends Record<string, unknown>>(
  source: T,
): Array<[string, unknown]> {
  return Object.entries(source).sort(([a], [b]) => String(a).localeCompare(String(b))) as Array<
    [string, unknown]
  >
}
