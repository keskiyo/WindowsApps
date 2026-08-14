import { toAppClientError } from '../../shared/api/tauri/errors'
import { reconcileFirstSeen, reconcileMarks } from './reconciliation'
import type { AppInfo, AppsClient } from '../../entities/app'
import type {
	AppState,
	GetAppState,
	PersistPreferences,
	SetAppState,
} from './types'

interface CatalogActionOptions {
	set: SetAppState
	get: GetAppState
	client: AppsClient
	persist: PersistPreferences
}

function errorMessage(error: unknown): string | null {
	const clientError = toAppClientError(error)
	return clientError.code === 'SCAN_CANCELLED' ? null : clientError.message
}

type CatalogActions = Pick<
	AppState,
	| 'load'
	| 'refresh'
	| 'forceFullScan'
	| 'resetCatalogCache'
	| 'cancelScan'
	| 'setQuery'
	| 'setActiveView'
>

export function createCatalogActions({
	set,
	get,
	client,
	persist,
}: CatalogActionOptions): CatalogActions {
	function commitScan(apps: AppInfo[]) {
		const previous = get().firstSeenAt
		const firstSeenAt = reconcileFirstSeen(apps, previous, Date.now())
		const marks = reconcileMarks(get(), apps)
		set({ apps, hasCache: true, firstSeenAt, ...marks })
		if (firstSeenAt !== previous || marks) persist()
	}

	return {
		async load() {
			set({ isLoading: true, error: null })
			try {
				const snapshot = await client.getApps()
				set({
					apps: snapshot.apps,
					firstSeenAt: reconcileFirstSeen(
						snapshot.apps,
						get().firstSeenAt,
						Date.now(),
					),
					hasCache: snapshot.hasCache,
					catalogGeneration: snapshot.generation ?? 0,
					catalogDiagnostics: snapshot.diagnostics ?? null,
					...reconcileMarks(get(), snapshot.apps),
				})
				persist()
			} catch (error) {
				set({ error: errorMessage(error) })
			} finally {
				set({ isLoading: false })
			}
		},
		async refresh() {
			set({ isRefreshing: true, error: null })
			try {
				commitScan(await client.refreshApps())
			} finally {
				set({ isRefreshing: false, scanProgress: null })
			}
		},
		async forceFullScan() {
			set({ isRefreshing: true, error: null })
			try {
				const apps = client.forceFullScan
					? await client.forceFullScan()
					: await client.refreshApps()
				commitScan(apps)
			} finally {
				set({ isRefreshing: false, scanProgress: null })
			}
		},
		async resetCatalogCache() {
			set({ isRefreshing: true, error: null })
			try {
				const apps = client.resetCatalogCache
					? await client.resetCatalogCache()
					: await get()
							.forceFullScan()
							.then(() => get().apps)
				commitScan(apps)
			} finally {
				set({ isRefreshing: false, scanProgress: null })
			}
		},
		async cancelScan() {
			await client.cancelScan()
		},
		setQuery(query) {
			set({ query })
		},
		setActiveView(activeView) {
			set({ activeView })
		},
	}
}
