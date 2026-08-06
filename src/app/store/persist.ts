import { writePreferences } from './preferences'
import type { GetAppState, PersistPreferences, SetAppState } from './types'

interface PersistOptions {
	set: SetAppState
	get: GetAppState
	storage: Storage
	unknownFields?: Record<string, unknown>
}

export function createPersist({
	set,
	get,
	storage,
	unknownFields,
}: PersistOptions): PersistPreferences {
	return () => {
		const state = get()
		const persisted = writePreferences(storage, {
			version: 12,
			categories: state.categories,
			categoryOrder: state.categoryOrder,
			favoriteAppIds: state.favoriteAppIds,
			favoriteAppIdentities: state.favoriteAppIdentities,
			collapsedCategories: state.collapsedCategories,
			categoryOverrides: state.categoryOverrides,
			categoryOverrideIdentities: state.categoryOverrideIdentities,
			hiddenAppIds: state.hiddenAppIds,
			hiddenAppIdentities: state.hiddenAppIdentities,
			promotedAppIds: state.promotedAppIds,
			promotedAppIdentities: state.promotedAppIdentities,
			installerAppIds: state.installerAppIds,
			installerAppIdentities: state.installerAppIdentities,
			scenarios: state.scenarios,
			firstSeenAt: state.firstSeenAt,
			legacyCanonicalPreferences: state.legacyCanonicalPreferences,
			unknownFields,
		})
		if (persisted !== state.preferencesPersisted) {
			set({ preferencesPersisted: persisted })
		}
	}
}
