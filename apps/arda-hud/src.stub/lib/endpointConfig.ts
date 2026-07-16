// sigil: REPAIR

export interface LoopbackEndpointOptions {
  scheme?: string
  host?: string
  port: string | number
}

export interface EnvEndpointOptions extends LoopbackEndpointOptions {
  url?: string | null
}

export function stripTrailingSlashes(value: string): string {
  return value.replace(/\/+$/, '')
}

export function loopbackUrl({ scheme = 'http', host = '127.0.0.1', port }: LoopbackEndpointOptions): string {
  return stripTrailingSlashes(`${scheme}://${host}:${port}`)
}

export function envString(value: unknown, fallback: string): string {
  return typeof value === 'string' && value.trim().length > 0 ? value.trim() : fallback
}

export function envEndpointUrl({ url, scheme = 'http', host = '127.0.0.1', port }: EnvEndpointOptions): string {
  return stripTrailingSlashes(envString(url, loopbackUrl({ scheme, host, port })))
}
