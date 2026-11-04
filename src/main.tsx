import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import { getCurrentWindow } from '@tauri-apps/api/window'
import SettingsView from './Settings'
import './index.css'

const isSettings = getCurrentWindow().label === 'settings'
if (isSettings) document.body.classList.add('settings-body')

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>{isSettings ? <SettingsView /> : <App />}</React.StrictMode>
)
