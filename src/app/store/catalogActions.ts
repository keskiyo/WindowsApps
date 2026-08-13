import { toAppClientError } from '../../shared/api/tauri/errors'
import {
	reconcileFirstSeen,
	reconcileOverrides,
	reconcileSelection,
} from './reconciliation'
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
		set({ apps, hasCache: true, firstSeenAt })
		if (firstSeenAt !== previous) persist()
	}

	return {
		async load() {
			set({ isLoading: true, error: null })
			try {
				const snapshot = await client.getApps()
				const legacy = get().legacyCanonicalPreferences
				const favorites = reconcileSelection(
					snapshot.apps,
					get().favoriteAppIds,
					get().favoriteAppIdentities,
					legacy.favorite,
				)
				const hidden = reconcileSelection(
					snapshot.apps,
					get().hiddenAppIds,
					get().hiddenAppIdentities,
					legacy.hidden,
				)
				const promoted = reconcileSelection(
					snapshot.apps,
					get().promotedAppIds,
					get().promotedAppIdentities,
					legacy.promoted,
				)
				const installers = reconcileSelection(
					snapshot.apps,
					get().installerAppIds,
					get().installerAppIdentities,
					legacy.installer,
				)
				const overrides = reconcileOverrides(
					snapshot.apps,
					get().categoryOverrides,
					get().categoryOverrideIdentities,
					legacy.categoryOverrides,
				)
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
					promotedAppIds: promoted.ids,
					promotedAppIdentities: promoted.identities,
					installerAppIds: installers.ids,
					installerAppIdentities: installers.identities,
					favoriteAppIds: favorites.ids,
					favoriteAppIdentities: favorites.identities,
					hiddenAppIds: hidden.ids,
					hiddenAppIdentities: hidden.identities,
					categoryOverrides: overrides.overrides,
					categoryOverrideIdentities: overrides.overrideIdentities,
					legacyCanonicalPreferences: {
						favorite: favorites.unresolvedLegacy,
						hidden: hidden.unresolvedLegacy,
						promoted: promoted.unresolvedLegacy,
						installer: installers.unresolvedLegacy,
						categoryOverrides: overrides.unresolvedLegacy,
					},
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
