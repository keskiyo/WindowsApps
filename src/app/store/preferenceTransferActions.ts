import {
	hasNewerStoredPreferences,
	parsePreferenceImport,
	readPreferenceBackup,
	serializePreferences,
	type AppPreferencesV14,
	type PreferenceTransferResult,
} from './preferences'
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

function preferencesFromState(state: AppState): AppPreferencesV14 {
	return {
		version: 14,
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
		favoriteScenarioIds: state.favoriteScenarioIds,
		scenarioHistory: state.scenarioHistory,
		firstSeenAt: state.firstSeenAt,
		legacyCanonicalPreferences: state.legacyCanonicalPreferences,
		unknownFields: state.unknownPreferenceFields,
	}
}

function preferenceState(preferences: AppPreferencesV14) {
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
		scenarios: preferences.scenarios,
		favoriteScenarioIds: preferences.favoriteScenarioIds,
		scenarioHistory: preferences.scenarioHistory,
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
	function applyPreferences(preferences: AppPreferencesV14): PreferenceTransferResult {
		if (hasNewerStoredPreferences(storage)) {
			return {
				ok: false,
				error: 'Settings cannot be replaced by this version of Windows Apps.',
			}
		}
		set(preferenceState(preferences))
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
