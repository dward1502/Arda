import { readdir, stat } from 'node:fs/promises'
import { extname, join, relative } from 'node:path'

const distRoot = new URL('../dist/assets/', import.meta.url)
const thresholds = {
  '.glb': 10 * 1024 * 1024,
  '.png': 4 * 1024 * 1024,
  '.jpg': 4 * 1024 * 1024,
  '.jpeg': 4 * 1024 * 1024,
  '.hdr': 4 * 1024 * 1024,
}
const totalLimit = 36 * 1024 * 1024

const entries = await readdir(distRoot, { withFileTypes: true })
const files = []
for (const entry of entries) {
  if (!entry.isFile()) continue
  const path = join(distRoot.pathname, entry.name)
  const info = await stat(path)
  files.push({ path, bytes: info.size, extension: extname(entry.name).toLowerCase() })
}

const violations = files.filter(({ bytes, extension }) => thresholds[extension] !== undefined && bytes > thresholds[extension])
const totalBytes = files.reduce((sum, file) => sum + file.bytes, 0)
if (totalBytes > totalLimit) {
  violations.push({ path: distRoot.pathname, bytes: totalBytes, extension: 'total' })
}

const report = {
  status: violations.length === 0 ? 'pass' : 'fail',
  totalBytes,
  totalLimit,
  checkedFiles: files.length,
  maxima: Object.fromEntries(Object.keys(thresholds).map((extension) => {
    const candidates = files.filter((file) => file.extension === extension)
    return [extension, candidates.length === 0 ? null : Math.max(...candidates.map((file) => file.bytes))]
  })),
  violations: violations.map((violation) => ({
    path: relative(distRoot.pathname, violation.path) || '.',
    bytes: violation.bytes,
    limit: violation.extension === 'total' ? totalLimit : thresholds[violation.extension],
  })),
}

console.log(JSON.stringify(report, null, 2))
if (violations.length > 0) process.exitCode = 1
