// sigil: REPAIR
interface SignalFieldProps {
  density?: 'low' | 'medium' | 'high'
  mode?: 'boardroom' | 'world'
}

const POINTS = [
  { x: '8%', y: '18%', size: 'small' },
  { x: '16%', y: '42%', size: 'medium' },
  { x: '24%', y: '30%', size: 'small' },
  { x: '38%', y: '16%', size: 'medium' },
  { x: '44%', y: '52%', size: 'small' },
  { x: '58%', y: '26%', size: 'small' },
  { x: '62%', y: '44%', size: 'large' },
  { x: '74%', y: '22%', size: 'small' },
  { x: '82%', y: '36%', size: 'medium' },
  { x: '88%', y: '18%', size: 'small' },
  { x: '70%', y: '62%', size: 'small' },
  { x: '26%', y: '66%', size: 'medium' },
]

const DENSITY_COUNT = {
  low: 6,
  medium: 9,
  high: POINTS.length,
} as const

export default function SignalField({
  density = 'medium',
  mode = 'boardroom',
}: SignalFieldProps) {
  const visiblePoints = POINTS.slice(0, DENSITY_COUNT[density])

  return (
    <div className={mode === 'world' ? 'signal-field signal-field--world' : 'signal-field signal-field--boardroom'}>
      <div className="signal-field__mesh" />
      {visiblePoints.map((point, index) => (
        <span
          key={`${mode}-${point.x}-${point.y}-${index}`}
          className={`signal-field__point signal-field__point--${point.size}`}
          style={{ left: point.x, top: point.y }}
        />
      ))}
    </div>
  )
}
