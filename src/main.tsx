import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from './App'
import { tauriSystemClient } from './lib/system'
import { tauriAppsClient } from './lib/tauri'
import { createAppStore } from './store/appStore'
import './index.css'

const appStore = createAppStore(tauriAppsClient)

createRoot(document.getElementById('root')!).render(
	<StrictMode>
		<App
			store={appStore}
			systemClient={tauriSystemClient}
			appsClient={tauriAppsClient}
		/>
	</StrictMode>,
)
