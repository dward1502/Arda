export type AppView = 'home' | 'boardroom' | 'dashboard'

export type AppAction =
  | { type: 'navigate'; view: AppView }
  | { type: 'openHermesDashboard' }
  | { type: 'backToBoardroom' }
