import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
	AppHydrationPatch,
	AppInfo,
	AppsClient,
	CatalogChangeSummary,
	CatalogDelta,
	CatalogSnapshot,
	LaunchStatus,
	ScanProgress,
	UninstallPreview,
} from '../types'

type TauriGlobal = typeof globalThis & {
	__TAURI_INTERNALS__?: unknown
}

function isTauriRuntime(): boolean {
	return Boolean((globalThis as TauriGlobal).__TAURI_INTERNALS__)
}

async function invokeIfTauri<T>(
	command: string,
	args?: Record<string, unknown>,
): Promise<T> {
	if (!isTauriRuntime())
		throw new Error('This action is available only in the desktop app.')
	return invoke<T>(command, args)
}

async function listenIfTauri<T>(
	event: string,
	handler: (payload: T) => void,
): Promise<() => void> {
	if (!isTauriRuntime()) return () => undefined
	return listen<T>(event, ({ payload }) => handler(payload))
}

export const tauriAppsClient: AppsClient = {
	getApps: () =>
		isTauriRuntime()
			? invoke<CatalogSnapshot>('get_apps')
			: Promise.resolve({ apps: [], hasCache: false }),
	refreshApps: () =>
		isTauriRuntime() ? invoke<AppInfo[]>('refresh_apps') : Promise.resolve([]),
	forceFullScan: () =>
		isTauriRuntime()
			? invoke<AppInfo[]>('force_full_scan')
			: Promise.resolve([]),
	resetCatalogCache: () =>
		isTauriRuntime()
			? invoke<AppInfo[]>('reset_catalog_cache')
			: Promise.resolve([]),
	hydrateVisibleIcons: ids =>
		isTauriRuntime()
			? invoke<void>('hydrate_visible_icons', { ids })
			: Promise.resolve(),
	startBackgroundSync: () =>
		isTauriRuntime()
			? invoke<void>('start_background_sync')
			: Promise.resolve(),
	cancelScan: () =>
		isTauriRuntime() ? invoke<void>('cancel_scan') : Promise.resolve(),
	launchApp: app => invokeIfTauri<void>('launch_app', { id: app.id }),
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
		return listenIfTauri<AppHydrationPatch[]>(
			'catalog://patches',
			handler,
		)
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
