import { useAppActions } from '../../hooks/useAppActions'
import BoardroomLayout from '../boardroom/BoardroomLayout'
import DashboardPanel from '../../features/dashboard/DashboardPanel'
import TopBar from './TopBar'

export default function ViewRouter() {
  const { state } = useAppActions()

  if (state.view === 'dashboard') {
    return (
      <div className="flex flex-col gap-4">
        <TopBar />
        <DashboardPanel />
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-4">
      <TopBar />
      <BoardroomLayout />
    </div>
  )
}
