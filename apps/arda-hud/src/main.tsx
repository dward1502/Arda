import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './styles/main.css'
import ViewRouter from './components/app/ViewRouter'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ViewRouter />
  </StrictMode>,
)
