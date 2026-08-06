import type { StoreApi } from 'zustand/vanilla'
import type { AppsClient } from '../entities/app'
import type { StaleCopyInfo, SystemClient } from '../entities/system'
import type { useUpdater } from '../features/update-app'
import type { AppState } from './store/appStore'

export interface AppProps {
	store: StoreApi<AppState>
	systemClient: SystemClient
	appsClient: Pick<AppsClient, 'getAppDetails' | 'openAppFolder'>
}

export interface AppShellChromeProps {
	activityActive: boolean
	activityLabel: string
	preferencesPersisted: boolean
	staleCopy: StaleCopyInfo | null
	systemClient: Pick<
		SystemClient,
		'openGithub' | 'openInstalledCopy' | 'openRelease'
	>
	updater: ReturnType<typeof useUpdater>
	onDismissStaleCopy: () => void
}

export interface GlobalActivityBarProps {
	active: boolean
	label: string
}
