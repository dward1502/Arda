interface SceneTransitionOverlayProps {
  active: boolean
  label: string
}

export default function SceneTransitionOverlay({ active, label }: SceneTransitionOverlayProps) {
  if (!active) return null

  return (
    <div className="scene-transition" aria-live="polite">
      <div className="scene-transition__beam" />
      <div className="scene-transition__frame">
        <span className="scene-transition__eyebrow">Scene Transition</span>
        <strong className="scene-transition__label">{label}</strong>
        <span className="scene-transition__hint">Routing executive presence through the city gate.</span>
      </div>
    </div>
  )
}
