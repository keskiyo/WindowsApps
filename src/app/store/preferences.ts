export { normalizePreferences } from './preferencesNormalize'
export {
	type AppPreferencesV16,
	CURRENT_PREFERENCES_VERSION,
	DEFAULT_PREFERENCES,
	type LegacyCanonicalPreferences,
	PREFERENCES_BACKUP_KEY,
	PREFERENCES_KEY,
	type PreferenceImportResult,
	type PreferenceTransferResult,
} from './preferencesSchema'
export {
	hasNewerStoredPreferences,
	parsePreferenceImport,
	readPreferenceBackup,
	readPreferences,
	serializePreferences,
	writePreferences,
} from './preferencesStorage'
