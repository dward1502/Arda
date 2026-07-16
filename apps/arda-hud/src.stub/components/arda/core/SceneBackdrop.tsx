// sigil: REPAIR
import SignalField from './SignalField'

interface SceneBackdropProps {
  mode: 'boardroom' | 'world'
  density?: 'low' | 'medium' | 'high'
}

export default function SceneBackdrop({ mode, density = 'medium' }: SceneBackdropProps) {
  return (
    <div className={mode === 'world' ? 'scene-backdrop scene-backdrop--world' : 'scene-backdrop scene-backdrop--boardroom'}>
      <SignalField mode={mode} density={density} />
      <div className="scene-backdrop__veil" />
    </div>
  )
}
