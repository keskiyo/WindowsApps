import type { StoreApi } from 'zustand/vanilla'
import type { AppInfo, AppsClient } from '../entities/app'
import type { CategoryDefinition } from '../entities/category'
import type { StaleCopyInfo, SystemClient } from '../entities/system'
import type { useUpdater } from '../features/update-app'
import type { useCatalogDialogs } from './model/useCatalogDialogs'
import type { AppState } from './store/appStore'

export interface AppProps {
	store: StoreApi<AppState>
	systemClient: SystemClient
	appsClient: Pick<
		AppsClient,
		'getAppDetails' | 'openAppFolder' | 'onCloseProgress'
	>
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

export interface AppDialogsProps {
	appsClient: Pick<AppsClient, 'getAppDetails' | 'openAppFolder'>
	categories: CategoryDefinition[]
	dialogs: ReturnType<typeof useCatalogDialogs>
	paletteApps: AppInfo[]
	paletteSuggestions: AppInfo[]
	onConfirmUninstall(): Promise<void>
	onError(kind: string, detail: string): void
}

export interface GlobalActivityBarProps {
	active: boolean
	label: string
}
