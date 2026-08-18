import {
	hasNewerStoredPreferences,
	parsePreferenceImport,
	readPreferenceBackup,
	serializePreferences,
	type AppPreferencesV16,
	type PreferenceTransferResult,
} from './preferences'
import { reconcileMarks } from './reconciliation'
import type {
	AppState,
	GetAppState,
	PersistPreferences,
	SetAppState,
} from './types'

interface PreferenceTransferOptions {
	set: SetAppState
	get: GetAppState
	persist: PersistPreferences
	storage: Storage
}

type PreferenceTransferActions = Pick<
	AppState,
	| 'exportPreferences'
	| 'validatePreferencesImport'
	| 'importPreferences'
	| 'restorePreferencesBackup'
>

function preferencesFromState(state: AppState): AppPreferencesV16 {
	return {
		version: 16,
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
		documentAppIds: state.documentAppIds,
		documentAppIdentities: state.documentAppIdentities,
		scenarios: state.scenarios,
		favoriteScenarioIds: state.favoriteScenarioIds,
		firstSeenAt: state.firstSeenAt,
		legacyCanonicalPreferences: state.legacyCanonicalPreferences,
		unknownFields: state.unknownPreferenceFields,
	}
}

function preferenceState(preferences: AppPreferencesV16) {
	return {
		categories: preferences.categories,
		categoryOrder: preferences.categoryOrder,
		favoriteAppIds: preferences.favoriteAppIds,
		favoriteAppIdentities: preferences.favoriteAppIdentities,
		collapsedCategories: preferences.collapsedCategories,
		categoryOverrides: preferences.categoryOverrides,
		categoryOverrideIdentities: preferences.categoryOverrideIdentities,
		hiddenAppIds: preferences.hiddenAppIds,
		hiddenAppIdentities: preferences.hiddenAppIdentities,
		promotedAppIds: preferences.promotedAppIds,
		promotedAppIdentities: preferences.promotedAppIdentities,
		installerAppIds: preferences.installerAppIds,
		installerAppIdentities: preferences.installerAppIdentities,
		documentAppIds: preferences.documentAppIds,
		documentAppIdentities: preferences.documentAppIdentities,
		scenarios: preferences.scenarios,
		favoriteScenarioIds: preferences.favoriteScenarioIds,
		firstSeenAt: preferences.firstSeenAt,
		legacyCanonicalPreferences: preferences.legacyCanonicalPreferences,
		unknownPreferenceFields: preferences.unknownFields ?? {},
	}
}

export function createPreferenceTransferActions({
	set,
	get,
	persist,
	storage,
}: PreferenceTransferOptions): PreferenceTransferActions {
	function applyPreferences(
		preferences: AppPreferencesV16,
	): PreferenceTransferResult {
		if (hasNewerStoredPreferences(storage)) {
			return {
				ok: false,
				error: 'Settings cannot be replaced by this version of Windows Apps.',
			}
		}
		set(preferenceState(preferences))
		set(reconcileMarks(get(), get().apps) ?? {})
		persist()
		return { ok: true }
	}

	return {
		exportPreferences() {
			return serializePreferences(preferencesFromState(get()))
		},
		validatePreferencesImport(source) {
			const result = parsePreferenceImport(source)
			return result.ok ? { ok: true } : result
		},
		importPreferences(source) {
			const result = parsePreferenceImport(source)
			return result.ok ? applyPreferences(result.preferences) : result
		},
		restorePreferencesBackup() {
			const result = readPreferenceBackup(storage)
			return result.ok ? applyPreferences(result.preferences) : result
		},
	}
}
