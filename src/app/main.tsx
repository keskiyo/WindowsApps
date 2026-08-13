import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { App } from './App'
import { AppErrorBoundary } from './AppErrorBoundary'
import { tauriAppsClient } from '../entities/app/api/appsClient'
import { tauriSystemClient } from '../entities/system/api/systemClient'
import { createAppStore } from './store/appStore'
import './styles/index.css'

const appStore = createAppStore(tauriAppsClient)

createRoot(document.getElementById('root')!).render(
	<StrictMode>
		<AppErrorBoundary>
			<App
				store={appStore}
				systemClient={tauriSystemClient}
				appsClient={tauriAppsClient}
			/>
		</AppErrorBoundary>
	</StrictMode>,
)
