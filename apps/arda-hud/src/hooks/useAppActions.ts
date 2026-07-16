import { useReducer, useCallback } from 'react'
import type { AppAction, AppView } from '../types/app-actions'

interface State {
  view: AppView
  lastAction: AppAction | null
}

const initialState: State = {
  view: 'boardroom',
  lastAction: null,
}

export function appReducer(state: State, action: AppAction): State {
  switch (action.type) {
    case 'navigate':
      return { ...state, view: action.view, lastAction: action }
    case 'openHermesDashboard':
      return { ...state, view: 'dashboard', lastAction: action }
    case 'backToBoardroom':
      return { ...state, view: 'boardroom', lastAction: action }
    default:
      return state
  }
}

export function useAppActions() {
  const [state, dispatch] = useReducer(appReducer, initialState)

  const navigate = useCallback((view: AppView) => dispatch({ type: 'navigate', view }), [])
  const openHermesDashboard = useCallback(() => dispatch({ type: 'openHermesDashboard' }), [])
  const backToBoardroom = useCallback(() => dispatch({ type: 'backToBoardroom' }), [])

  return { state, dispatch, navigate, openHermesDashboard, backToBoardroom }
}
