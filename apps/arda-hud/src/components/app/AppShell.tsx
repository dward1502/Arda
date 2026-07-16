import type { AppView } from '../../types/app-actions'

export function switchView(navigate: (view: AppView) => void, view: Exclude<AppView, 'home'>) {
  navigate(view)
}
