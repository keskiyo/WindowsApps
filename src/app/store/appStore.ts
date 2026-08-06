import { createStore, type StoreApi } from 'zustand/vanilla'
import { createAppMarkActions } from './appMarkActions'
import { createAppPlacementActions } from './appPlacementActions'
import { createCatalogActions } from './catalogActions'
import { createCatalogSyncActions } from './catalogSyncActions'
import { createCategoryActions } from './categoryActions'
import { createIconActions } from './iconActions'
import { createLaunchActions } from './launchActions'
import { createLifecycleActions } from './lifecycleActions'
import { createPersist } from './persist'
import { createScenarioActions } from './scenarioActions'
import { readPreferences } from './preferences'
import type { AppPreferencesV12 } from './preferences'
import type { AppsClient } from '../../entities/app'
import type { AppState } from './types'

/** The persisted half of the initial state; the rest is runtime-only and always starts empty. */
function initialState(preferences: AppPreferencesV12) {
	return {
		apps: [],
		query: '',
		isLoading: true,
		isRefreshing: false,
		scanProgress: null,
		hasCache: false,
		catalogGeneration: 0,
		catalogChange: null,
		catalogDiagnostics: null,
		error: null,
		activeView: 'all' as const,
		favoriteAppIds: preferences.favoriteAppIds,
		favoriteAppIdentities: preferences.favoriteAppIdentities,
		categoryOrder: preferences.categoryOrder,
		collapsedCategories: preferences.collapsedCategories,
		categoryOverrides: preferences.categoryOverrides,
		categoryOverrideIdentities: preferences.categoryOverrideIdentities,
		hiddenAppIds: preferences.hiddenAppIds,
		hiddenAppIdentities: preferences.hiddenAppIdentities,
		promotedAppIds: preferences.promotedAppIds,
		promotedAppIdentities: preferences.promotedAppIdentities,
		installerAppIds: preferences.installerAppIds,
		installerAppIdentities: preferences.installerAppIdentities,
		scenarios: preferences.scenarios,
		firstSeenAt: preferences.firstSeenAt,
		legacyCanonicalPreferences: preferences.legacyCanonicalPreferences,
		preferencesPersisted: true,
		categories: preferences.categories,
		launchingIds: [],
	}
}

/**
 * The single root store. It owns no behaviour itself: every action group is a focused module in
 * this folder, assembled here against one `set`/`get` and one `persist`. Adding a domain means
 * adding a module and one line below, never another branch in this file.
 */
export function createAppStore(
	client: AppsClient,
	storage: Storage = globalThis.localStorage,
	idFactory: () => string = () => `custom:${crypto.randomUUID()}`,
): StoreApi<AppState> {
	const preferences = readPreferences(storage)
	return createStore<AppState>((set, get) => {
		const persist = createPersist({
			set,
			get,
			storage,
			unknownFields: preferences.unknownFields,
		})
		return {
			...initialState(preferences),
			...createLifecycleActions({ set, get, client }),
			...createCatalogActions({ set, get, client, persist }),
			...createCatalogSyncActions({ set, get, client, persist }),
			...createIconActions({ get, client }),
			...createLaunchActions({ set, get, client }),
			...createAppMarkActions({ set, get, persist }),
			...createAppPlacementActions({ set, persist }),
			...createCategoryActions({ set, get, persist, idFactory }),
			...createScenarioActions({ set, get, persist, idFactory }),
		}
	})
}

export type { AppState } from './types'

// Pure selectors/filters live in ./selectors. Re-exported here so existing imports from
// '../store/appStore' keep working after the split.
export {
	filterAppsByQuery,
	filterVisibleApps,
	rankAppsByQuery,
	rankAppsByQueryTop,
	selectCatalogCounts,
	selectCategorizedApps,
	selectFilteredApps,
	selectVisibleApps,
} from './selectors'
