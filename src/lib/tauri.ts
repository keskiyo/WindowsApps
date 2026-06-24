import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
	AppInfo,
	AppHydrationPatch,
	AppsClient,
	CatalogChangeSummary,
	CatalogDelta,
	CatalogSnapshot,
	ScanProgress,
	UninstallPreview,
} from '../types'

export const tauriAppsClient: AppsClient = {
	getApps: () => invoke<CatalogSnapshot>('get_apps'),
	refreshApps: () => invoke<AppInfo[]>('refresh_apps'),
	forceFullScan: () => invoke<AppInfo[]>('force_full_scan'),
	resetCatalogCache: () => invoke<AppInfo[]>('reset_catalog_cache'),
	hydrateVisibleIcons: ids => invoke<void>('hydrate_visible_icons', { ids }),
	startBackgroundSync: () => invoke<void>('start_background_sync'),
	cancelScan: () => invoke<void>('cancel_scan'),
	launchApp: app => invoke<void>('launch_app', { id: app.id }),
	getUninstallPreview: id =>
		invoke<UninstallPreview>('get_uninstall_preview', { id }),
	uninstallApp: id => invoke<void>('uninstall_app', { id }),
	async onAppsUpdated(handler) {
		return listen<AppInfo[]>('apps://updated', ({ payload }) =>
			handler(payload),
		)
	},
	async onCatalogDelta(handler) {
		return listen<CatalogDelta>('catalog://delta', ({ payload }) =>
			handler(payload),
		)
	},
	async onCatalogPatches(handler) {
		return listen<AppHydrationPatch[]>('catalog://patches', ({ payload }) =>
			handler(payload),
		)
	},
	async onCatalogChanged(handler) {
		return listen<CatalogChangeSummary>(
			'catalog://changed',
			({ payload }) => handler(payload),
		)
	},
	async onScanProgress(handler) {
		return listen<ScanProgress>('scan://progress', ({ payload }) =>
			handler(payload),
		)
	},
}
