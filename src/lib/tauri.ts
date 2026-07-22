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

export type AppErrorCode =
	| 'APP_DATA_UNAVAILABLE'
	| 'CLEAR_ICON_CACHE_FAILED'
	| 'CLEAR_UNINSTALL_HISTORY_FAILED'
	| 'DESKTOP_RUNTIME_UNAVAILABLE'
	| 'INTERNAL'
	| 'INVALID_RELEASE_VERSION'
	| 'LAUNCH_DATA_UNAVAILABLE'
	| 'LAUNCH_UNAVAILABLE'
	| 'NO_NEWER_COPY'
	| 'OPERATION_FAILED'
	| 'OPERATION_INTERRUPTED'
	| 'PRODUCT_NAME_MISSING'
	| 'RESET_CATALOG_CACHE_FAILED'
	| 'RESET_ICON_CACHE_FAILED'
	| 'SAVE_SCAN_SETTINGS_FAILED'
	| 'SCAN_COALESCED'
	| 'SCAN_PATH_NOT_ABSOLUTE'
	| 'UNINSTALL_DATA_UNAVAILABLE'
	| 'UNINSTALL_UNAVAILABLE'

interface AppErrorPayload {
	code: AppErrorCode
	message: string
}

export class AppClientError extends Error {
	constructor(
		readonly code: AppErrorCode,
		message: string,
	) {
		super(message)
		this.name = 'AppClientError'
	}
}

type TauriGlobal = typeof globalThis & {
	__TAURI_INTERNALS__?: unknown
}

function isTauriRuntime(): boolean {
	return Boolean((globalThis as TauriGlobal).__TAURI_INTERNALS__)
}

function isAppErrorCode(value: unknown): value is AppErrorCode {
	return typeof value === 'string' && value in APP_ERROR_CODES
}

const APP_ERROR_CODES = {
	APP_DATA_UNAVAILABLE: true,
	CLEAR_ICON_CACHE_FAILED: true,
	CLEAR_UNINSTALL_HISTORY_FAILED: true,
	DESKTOP_RUNTIME_UNAVAILABLE: true,
	INTERNAL: true,
	INVALID_RELEASE_VERSION: true,
	LAUNCH_DATA_UNAVAILABLE: true,
	LAUNCH_UNAVAILABLE: true,
	NO_NEWER_COPY: true,
	OPERATION_FAILED: true,
	OPERATION_INTERRUPTED: true,
	PRODUCT_NAME_MISSING: true,
	RESET_CATALOG_CACHE_FAILED: true,
	RESET_ICON_CACHE_FAILED: true,
	SAVE_SCAN_SETTINGS_FAILED: true,
	SCAN_COALESCED: true,
	SCAN_PATH_NOT_ABSOLUTE: true,
	UNINSTALL_DATA_UNAVAILABLE: true,
	UNINSTALL_UNAVAILABLE: true,
} as const

function readAppErrorPayload(value: unknown): AppErrorPayload | null {
	if (!value || typeof value !== 'object') return null
	const candidate = value as { code?: unknown; message?: unknown }
	return isAppErrorCode(candidate.code) && typeof candidate.message === 'string'
		? { code: candidate.code, message: candidate.message }
		: null
}

export function toAppClientError(error: unknown): AppClientError {
	if (error instanceof AppClientError) return error
	const directPayload = readAppErrorPayload(error)
	if (directPayload)
		return new AppClientError(directPayload.code, directPayload.message)
	if (typeof error === 'string') {
		try {
			const payload = readAppErrorPayload(JSON.parse(error))
			if (payload) return new AppClientError(payload.code, payload.message)
		} catch {
			// Unknown values must not surface raw transport details in the interface.
		}
	}
	return new AppClientError('INTERNAL', 'The operation could not be completed. Try again.')
}

export async function invokeTauri<T>(
	command: string,
	args?: Record<string, unknown>,
): Promise<T> {
	try {
		return await invoke<T>(command, args)
	} catch (error) {
		throw toAppClientError(error)
	}
}

async function invokeIfTauri<T>(
	command: string,
	args?: Record<string, unknown>,
): Promise<T> {
	if (!isTauriRuntime())
		throw new AppClientError(
			'DESKTOP_RUNTIME_UNAVAILABLE',
			'This action is available only in the desktop app.',
		)
	return invokeTauri<T>(command, args)
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
			? invokeTauri<CatalogSnapshot>('get_apps')
			: Promise.resolve({ apps: [], hasCache: false }),
	refreshApps: () =>
		isTauriRuntime() ? invokeTauri<AppInfo[]>('refresh_apps') : Promise.resolve([]),
	forceFullScan: () =>
		isTauriRuntime()
			? invokeTauri<AppInfo[]>('force_full_scan')
			: Promise.resolve([]),
	resetCatalogCache: () =>
		isTauriRuntime()
			? invokeTauri<AppInfo[]>('reset_catalog_cache')
			: Promise.resolve([]),
	clearIconCache: () =>
		isTauriRuntime() ? invokeTauri<void>('clear_icon_cache') : Promise.resolve(),
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
