import { normalizePreferences } from './preferencesNormalize'
import {
	type AppPreferencesV16,
	CURRENT_PREFERENCES_VERSION,
	PREFERENCES_BACKUP_KEY,
	PREFERENCES_KEY,
	type PreferenceImportResult,
} from './preferencesSchema'

export function serializePreferences(preferences: AppPreferencesV16): string {
	const { unknownFields = {}, ...knownFields } = preferences
	return JSON.stringify({ ...unknownFields, ...knownFields })
}

export function parsePreferenceImport(source: string): PreferenceImportResult {
	try {
		const value: unknown = JSON.parse(source)
		if (!value || typeof value !== 'object' || Array.isArray(value)) {
			return {
				ok: false,
				error: 'The selected file is not a Windows Apps backup.',
			}
		}
		const version = (value as { version?: unknown }).version
		if (
			typeof version !== 'number' ||
			!Number.isSafeInteger(version) ||
			version < 1
		) {
			return {
				ok: false,
				error: 'The selected file is not a Windows Apps backup.',
			}
		}
		if (version > CURRENT_PREFERENCES_VERSION) {
			return {
				ok: false,
				error: 'This backup was created by a newer version of Windows Apps.',
			}
		}
		return { ok: true, preferences: normalizePreferences(value) }
	} catch {
		return {
			ok: false,
			error: 'The selected file is not a Windows Apps backup.',
		}
	}
}

export function readPreferenceBackup(storage: Storage): PreferenceImportResult {
	try {
		const source = storage.getItem(PREFERENCES_BACKUP_KEY)
		if (!source)
			return { ok: false, error: 'No local backup is available yet.' }
		return parsePreferenceImport(source)
	} catch {
		return { ok: false, error: 'The local backup could not be read.' }
	}
}

export function readPreferences(storage: Storage): AppPreferencesV16 {
	return (
		readSlot(storage, PREFERENCES_KEY) ??
		readSlot(storage, PREFERENCES_BACKUP_KEY) ??
		normalizePreferences(null)
	)
}

function readSlot(storage: Storage, key: string): AppPreferencesV16 | null {
	try {
		const value = storage.getItem(key)
		return value ? normalizePreferences(JSON.parse(value)) : null
	} catch {
		return null
	}
}

export function writePreferences(
	storage: Storage,
	preferences: AppPreferencesV16,
): boolean {
	if (hasNewerStoredPreferences(storage)) {
		return true
	}
	rotateBackup(storage)
	try {
		storage.setItem(PREFERENCES_KEY, serializePreferences(preferences))
		return true
	} catch {
		return false
	}
}

export function hasNewerStoredPreferences(storage: Storage): boolean {
	try {
		const raw = storage.getItem(PREFERENCES_KEY)
		if (!raw) return false
		const version = (JSON.parse(raw) as { version?: unknown }).version
		return (
			typeof version === 'number' && version > CURRENT_PREFERENCES_VERSION
		)
	} catch {
		return false
	}
}

function rotateBackup(storage: Storage): void {
	try {
		const current = storage.getItem(PREFERENCES_KEY)
		if (current) storage.setItem(PREFERENCES_BACKUP_KEY, current)
	} catch (ignored) {
		void ignored
	}
}
