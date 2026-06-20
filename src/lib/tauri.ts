import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
	AppInfo,
	AppsClient,
	CatalogSnapshot,
	ScanProgress,
} from '../types'

export const tauriAppsClient: AppsClient = {
	getApps: () => invoke<CatalogSnapshot>('get_apps'),
	refreshApps: () => invoke<AppInfo[]>('refresh_apps'),
	cancelScan: () => invoke<void>('cancel_scan'),
	launchApp: app =>
		invoke<void>('launch_app', {
			launchKind: app.launchKind,
			path: app.path,
		}),
	uninstallApp: id => invoke<void>('uninstall_app', { id }),
	async onAppsUpdated(handler) {
		return listen<AppInfo[]>('apps://updated', ({ payload }) =>
			handler(payload),
		)
	},
	async onScanProgress(handler) {
		return listen<ScanProgress>('scan://progress', ({ payload }) =>
			handler(payload),
		)
	},
}
