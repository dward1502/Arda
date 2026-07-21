import React from 'react'

interface ArdaErrorBoundaryProps {
  children: React.ReactNode
}

interface ArdaErrorBoundaryState {
  hasError: boolean
  message: string
  attemptedReload: boolean
}

export default class ArdaErrorBoundary extends React.Component<ArdaErrorBoundaryProps, ArdaErrorBoundaryState> {
  constructor(props: ArdaErrorBoundaryProps) {
    super(props)
    this.state = { hasError: false, message: '', attemptedReload: false }
  }

  static getDerivedStateFromError(error: unknown): Pick<ArdaErrorBoundaryState, 'hasError' | 'message' | 'attemptedReload'> {
    return {
      hasError: true,
      message: error instanceof Error ? error.message : 'Unknown render failure',
      attemptedReload: false,
    }
  }

  componentDidCatch(error: unknown, info: React.ErrorInfo) {
    console.error('ARDA_HUD render failure', error, info)
  }

  reloadSurface = () => {
    if ((this.state as ArdaErrorBoundaryState & { attemptedReload?: boolean }).attemptedReload) {
      return
    }
    this.setState({ attemptedReload: true } as Partial<ArdaErrorBoundaryState> as ArdaErrorBoundaryState)
    window.location.reload()
  }

  render() {
    const { hasError, message } = this.state
    if (hasError) {
      return (
        <div className="arda-failsafe">
          <div className="arda-failsafe__eyebrow">Failsafe Surface</div>
          <h1>ARDA_HUD recovered from a render fault.</h1>
          <p>{message}</p>
          <button
            className="arda-failsafe__button"
            onClick={this.reloadSurface}
            type="button"
          >
            Reload Surface
          </button>
        </div>
      )
    }

    return this.props.children
  }
}
