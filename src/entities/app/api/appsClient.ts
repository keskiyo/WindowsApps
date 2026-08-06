import type {
	AppDetails,
	AppHydrationPatch,
	AppInfo,
	AppsClient,
	CatalogChangeSummary,
	CatalogDelta,
	CatalogSnapshot,
	CloseAppsResult,
	LaunchStatus,
	ScanProgress,
	UninstallPreview,
} from '../model/app.types'
import {
	invokeIfTauri,
	invokeTauri,
	isTauriRuntime,
	listenIfTauri,
} from '../../../shared/api/tauri/client'

export const tauriAppsClient: AppsClient = {
	getApps: () =>
		isTauriRuntime()
			? invokeTauri<CatalogSnapshot>('get_apps')
			: Promise.resolve({ apps: [], hasCache: false }),
	refreshApps: () =>
		isTauriRuntime()
			? invokeTauri<AppInfo[]>('refresh_apps')
			: Promise.resolve([]),
	forceFullScan: () =>
		isTauriRuntime()
			? invokeTauri<AppInfo[]>('force_full_scan')
			: Promise.resolve([]),
	resetCatalogCache: () =>
		isTauriRuntime()
			? invokeTauri<AppInfo[]>('reset_catalog_cache')
			: Promise.resolve([]),
	clearIconCache: () =>
		isTauriRuntime()
			? invokeTauri<void>('clear_icon_cache')
			: Promise.resolve(),
	hydrateVisibleIcons: ids =>
		isTauriRuntime()
			? invokeTauri<void>('hydrate_visible_icons', { ids })
			: Promise.resolve(),
	startBackgroundSync: () =>
		isTauriRuntime()
			? invokeTauri<void>('start_background_sync')
			: Promise.resolve(),
	cancelScan: () =>
		isTauriRuntime() ? invokeTauri<void>('cancel_scan') : Promise.resolve(),
	launchApp: app => invokeIfTauri<void>('launch_app', { id: app.id }),
	closeApps: ids => invokeIfTauri<CloseAppsResult>('close_apps', { ids }),
	getAppDetails: id => invokeIfTauri<AppDetails>('get_app_details', { id }),
	openAppFolder: id => invokeIfTauri<void>('open_app_folder', { id }),
	getUninstallPreview: id =>
		invokeIfTauri<UninstallPreview>('get_uninstall_preview', { id }),
	uninstallApp: id => invokeIfTauri<void>('uninstall_app', { id }),
	async onAppsUpdated(handler) {
		return listenIfTauri<AppInfo[]>('apps://updated', handler)
	},
	async onCatalogDelta(handler) {
		return listenIfTauri<CatalogDelta>('catalog://delta', handler)
	},
	async onCatalogPatches(handler) {
		return listenIfTauri<AppHydrationPatch[]>('catalog://patches', handler)
	},
	async onCatalogChanged(handler) {
		return listenIfTauri<CatalogChangeSummary>('catalog://changed', handler)
	},
	async onScanProgress(handler) {
		return listenIfTauri<ScanProgress>('scan://progress', handler)
	},
	async onLaunchStatus(handler) {
		return listenIfTauri<LaunchStatus>('launch://status', handler)
	},
}
