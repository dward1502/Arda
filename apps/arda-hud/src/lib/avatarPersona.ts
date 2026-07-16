// sigil: REPAIR

interface AvatarEventLike {
  severity?: string
  message: string
}

interface AvatarSystemNodeLike {
  status?: string
}

export type AvatarPersona = 'frankyrache' | 'rache' | 'bartmoss'
export type AvatarMood = 'success' | 'warning' | 'error' | 'deploying'
export type AvatarLineType = 'success' | 'error' | 'confirm' | 'status'

export const AVATAR_PERSONAS: Array<{ id: AvatarPersona; label: string }> = [
  { id: 'frankyrache', label: 'Franky-Rache' },
  { id: 'rache', label: 'Rache Paranoid' },
  { id: 'bartmoss', label: 'Bartmoss Ghost' },
]

export const AVATAR_MOOD_STYLES: Record<AvatarMood, { glow: string; aura: string; accent: string; status: string }> = {
  success: { glow: '#00ff9f', aura: '#00ff9f', accent: '#00f0ff', status: 'Nominal' },
  warning: { glow: '#ff9933', aura: '#ff9933', accent: '#ff6600', status: 'Warning' },
  error: { glow: '#ff2a6d', aura: '#ff2a6d', accent: '#ff2a6d', status: 'Critical' },
  deploying: { glow: '#00f0ff', aura: '#00f0ff', accent: '#00d4ff', status: 'Deploying' },
}

const AVATAR_LINES: Record<AvatarPersona, Record<AvatarLineType, Record<AvatarMood, string[]>>> = {
  frankyrache: {
    success: {
      success: ['SUUUUPER clean, choom. Systems look crisp.', 'Done, choom. Flow is nominal.'],
      warning: ['Handled, but keep eyes on amber lanes.', 'Done. Keep it tight, warning still live.'],
      error: ['Action landed, but red heat remains.', 'Done. We still got critical fire nearby.'],
      deploying: ['Deploying clean. Super sync in motion.', 'Done. Build energy is moving.'],
    },
    error: {
      success: ['That did not land, choom. Retrying might fix it.', 'No go on that command.'],
      warning: ['Failed under warning load. Check path and perms.', 'Negative result. We hit friction.'],
      error: ['Critical fail. We need a safer route.', 'No joy. Red lane blocked the action.'],
      deploying: ['Deploy phase conflict. Command failed.', 'Nope. Build lane rejected that move.'],
    },
    confirm: {
      success: ['Dangerous move, choom. Say YES to light the fuse.'],
      warning: ['High risk command. Confirm with YES.'],
      error: ['Critical conditions active. Say YES if you really mean it.'],
      deploying: ['Deploy lane active. Confirm with YES before execution.'],
    },
    status: {
      success: ['Status nominal. Ready for next command.'],
      warning: ['Status caution. Recommend log sweep next.'],
      error: ['Status critical. Suggest immediate containment.'],
      deploying: ['Status deploying. Monitoring sync.'],
    },
  },
  rache: {
    success: {
      success: ['Quiet net. Command complete.', 'Done. Keep one eye on the shadows.'],
      warning: ['Done, but someone is still watching.', 'Action complete in noisy lanes.'],
      error: ['Completed under fire. Stay sharp.', 'Done. We are still in hostile space.'],
      deploying: ['Deploy action done. Ghosting traffic now.', 'Done. Routing through contested lanes.'],
    },
    error: {
      success: ['Command failed. The wire pushed back.', 'No execution. Check permissions and route.'],
      warning: ['Failure under warning conditions.', 'Blocked. This smells like ICE.'],
      error: ['Hard fail in critical lane. Lock down and retry.', 'No go. Red signal dominated the path.'],
      deploying: ['Deploy conflict. Action rejected.', 'Failed while sync channel was hot.'],
    },
    confirm: {
      success: ['Are you certain? Say YES to proceed.'],
      warning: ['Warning trace detected. Say YES to continue.'],
      error: ['Critical zone active. Confirm with YES only if intentional.'],
      deploying: ['Deploy route active. Say YES to confirm.'],
    },
    status: {
      success: ['Status nominal... for now.'],
      warning: ['Status warning. Probe carefully.'],
      error: ['Status critical. Recommend immediate containment.'],
      deploying: ['Status deploying. Ghostline stable.'],
    },
  },
  bartmoss: {
    success: {
      success: ['Grid purrs. Command executed.', 'Done. Daemons are happy.'],
      warning: ['Done with orange static still present.', 'Action complete. Mesh remains unstable.'],
      error: ['Executed under red storm.', 'Done, but the backbone is still screaming.'],
      deploying: ['Deploy command landed. Ghostwave active.', 'Done. Payload riding the mesh.'],
    },
    error: {
      success: ['Command bounced. Try a cleaner vector.', 'No output. Need a better route.'],
      warning: ['Failure in warning noise.', 'Failed. Mesh interference too high.'],
      error: ['Critical break. Cut path and reroute.', 'Hard fail. Black ICE in the lane.'],
      deploying: ['Deploy lane rejected that action.', 'Failed during payload transit.'],
    },
    confirm: {
      success: ['High impact command. Say YES to proceed.'],
      warning: ['Orange static up. Confirm with YES.'],
      error: ['Red storm active. Say YES only if committed.'],
      deploying: ['Payload active. Confirm with YES to continue.'],
    },
    status: {
      success: ['Status green. Mesh stable.'],
      warning: ['Status amber. Keep telemetry open.'],
      error: ['Status red. Immediate mitigation advised.'],
      deploying: ['Status cyan. Deployment in progress.'],
    },
  },
}

export function computeAvatarMood(
  events: AvatarEventLike[],
  systemNodes: AvatarSystemNodeLike[],
  connectionStatus: 'connected' | 'disconnected' | 'connecting'
): AvatarMood {
  const latest = events[0]
  if (connectionStatus !== 'connected' || latest?.severity === 'critical' || latest?.severity === 'error') {
    return 'error'
  }
  if (latest?.severity === 'warn' || systemNodes.some((node) => node.status === 'degraded' || node.status === 'offline')) {
    return 'warning'
  }
  if (latest && /(deploy|sync|bundle|build)/i.test(latest.message)) {
    return 'deploying'
  }
  return 'success'
}

export function getAvatarLine(persona: AvatarPersona, mood: AvatarMood, type: AvatarLineType): string {
  const pool = AVATAR_LINES[persona][type][mood]
  return pool[Math.floor(Math.random() * pool.length)]
}
