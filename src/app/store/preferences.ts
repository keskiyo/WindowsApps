import {
	CATEGORY_ORDER,
	DEFAULT_CATEGORIES,
	type AppCategory,
	type CategoryDefinition,
} from '../types'
import { isCustomCategoryAccent, stableCustomCategoryAccent } from './categoryAccents'

export const PREFERENCES_KEY = 'windows-apps.preferences.v1'

/** The schema version this build understands. `version` in the stored document is independent
 * of the key name; see `AGENTS_frontend.md` §3. */
export const CURRENT_PREFERENCES_VERSION = 8

/**
 * Previous known-good copy. The backend cache has kept a `.bak` and recovered from it since the
 * beginning; this store had nothing, so a value that failed to parse — a partial write, a
 * hand-edit, anything — silently became "no favorites, nothing hidden, default categories" with
 * no way back.
 */
export const PREFERENCES_BACKUP_KEY = 'windows-apps.preferences.v1.bak'

export interface LegacyCanonicalPreferences {
	favorite: string[]
	hidden: string[]
	promoted: string[]
	categoryOverrides: Record<string, AppCategory>
}

export interface AppPreferencesV8 {
	version: 8
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	// `*AppIds` are catalog ids: durable within a version, but an id is a function of the
	// deduplication grouping, so it can change between releases. `*AppIdentities` carry the
	// stable `canonicalIdentity`, which is what actually survives a dedup rule change — the
	// ids are re-derived from them on load (see the store). The id arrays/maps are kept for the
	// one-time migration of preferences written before the identities existed, exactly as
	// `promotedAppIds` is kept beside `promotedAppIdentities`.
	favoriteAppIds: string[]
	favoriteAppIdentities: string[]
	collapsedCategories: AppCategory[]
	// A manual category override keyed by catalog id (runtime projection) and by the durable
	// `canonicalIdentity` (`categoryOverrideIdentities`). The identity map is what survives a
	// Force full scan / Reset cache / dedup rule change; the id map is re-derived from it on load.
	categoryOverrides: Record<string, AppCategory>
	categoryOverrideIdentities: Record<string, AppCategory>
	hiddenAppIds: string[]
	hiddenAppIdentities: string[]
	promotedAppIds: string[]
	promotedAppIdentities: string[]
	legacyCanonicalPreferences: LegacyCanonicalPreferences
	unknownFields?: Record<string, unknown>
}

export const DEFAULT_PREFERENCES: AppPreferencesV8 = {
	version: 8,
	categories: DEFAULT_CATEGORIES.map(category => ({ ...category })),
	categoryOrder: [...CATEGORY_ORDER],
	favoriteAppIds: [],
	favoriteAppIdentities: [],
	collapsedCategories: [],
	categoryOverrides: {},
	categoryOverrideIdentities: {},
	hiddenAppIds: [],
	hiddenAppIdentities: [],
	promotedAppIds: [],
	promotedAppIdentities: [],
	legacyCanonicalPreferences: {
		favorite: [],
		hidden: [],
		promoted: [],
		categoryOverrides: {},
	},
}

function uniqueStrings(value: unknown): string[] {
	return Array.isArray(value)
		? [
				...new Set(
					value.filter(
						(item): item is string =>
							typeof item === 'string' && item.trim().length > 0,
					),
				),
			]
		: []
}

/** A `{ key -> category }` map, keeping only entries with a non-empty key and a known category. */
function normalizeOverrideMap(
	value: unknown,
	known: Set<string>,
): Record<string, AppCategory> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return {}
	return Object.fromEntries(
		Object.entries(value as Record<string, unknown>).filter(
			([key, category]) =>
				key.trim() &&
				typeof category === 'string' &&
				known.has(category),
		),
	) as Record<string, AppCategory>
}

function normalizeDefinitions(value: unknown): CategoryDefinition[] {
	const saved = Array.isArray(value) ? value : []
	const labels = new Set<string>()
	const categories = DEFAULT_CATEGORIES.map(category => {
		const match = saved.find(
			item =>
				item &&
				typeof item === 'object' &&
				(item as { id?: unknown }).id === category.id,
		) as { label?: unknown } | undefined
		const label =
			typeof match?.label === 'string' && match.label.trim()
				? match.label.trim()
				: category.label
		labels.add(label.toLocaleLowerCase())
		return { ...category, label }
	})
	for (const item of saved) {
		if (!item || typeof item !== 'object') continue
		const raw = item as Record<string, unknown>
		const id = typeof raw.id === 'string' ? raw.id.trim() : ''
		const label = typeof raw.label === 'string' ? raw.label.trim() : ''
		if (
			!id.startsWith('custom:') ||
			!label ||
			labels.has(label.toLocaleLowerCase())
		)
			continue
		categories.push({
			id,
			label,
			builtIn: false,
			accent: isCustomCategoryAccent(raw.accent)
				? raw.accent
				: stableCustomCategoryAccent(id),
		})
		labels.add(label.toLocaleLowerCase())
	}
	return categories
}

const KNOWN_PREFERENCE_FIELDS = new Set([
	'version',
	'categories',
	'categoryOrder',
	'favoriteAppIds',
	'favoriteAppIdentities',
	'collapsedCategories',
	'categoryOverrides',
	'categoryOverrideIdentities',
	'hiddenAppIds',
	'hiddenAppIdentities',
	'promotedAppIds',
	'promotedAppIdentities',
	'legacyCanonicalPreferences',
])

export function normalizePreferences(value: unknown): AppPreferencesV8 {
	if (!value || typeof value !== 'object')
		return structuredClone(DEFAULT_PREFERENCES)
	const raw = value as Record<string, unknown>
	const categories = normalizeDefinitions(raw.categories)
	const known = new Set(categories.map(category => category.id))
	const savedOrder = uniqueStrings(raw.categoryOrder).filter(id =>
		known.has(id),
	)
	const categoryOrder = [
		...savedOrder,
		...categories
			.map(category => category.id)
			.filter(id => !savedOrder.includes(id)),
	]
	const overrides = normalizeOverrideMap(raw.categoryOverrides, known)
	const overrideIdentities = normalizeOverrideMap(
		raw.categoryOverrideIdentities,
		known,
	)
	const hasDurableIdentities = raw.version === 7 || raw.version === 8
	const rawLegacy =
		hasDurableIdentities &&
		raw.legacyCanonicalPreferences &&
		typeof raw.legacyCanonicalPreferences === 'object' &&
		!Array.isArray(raw.legacyCanonicalPreferences)
			? (raw.legacyCanonicalPreferences as Record<string, unknown>)
			: {}
	const legacyCanonicalPreferences: LegacyCanonicalPreferences = {
		favorite: uniqueStrings(
			hasDurableIdentities
				? rawLegacy.favorite
				: raw.favoriteAppIdentities,
		),
		hidden: uniqueStrings(
			hasDurableIdentities ? rawLegacy.hidden : raw.hiddenAppIdentities,
		),
		promoted: uniqueStrings(
			hasDurableIdentities
				? rawLegacy.promoted
				: raw.promotedAppIdentities,
		),
		categoryOverrides: normalizeOverrideMap(
			hasDurableIdentities
				? rawLegacy.categoryOverrides
				: raw.categoryOverrideIdentities,
			known,
		),
	}
	const unknownFields = Object.fromEntries(
		Object.entries(raw).filter(
			([key]) => !KNOWN_PREFERENCE_FIELDS.has(key),
		),
	)
	return {
		version: 8,
		categories,
		categoryOrder,
		favoriteAppIds: uniqueStrings(raw.favoriteAppIds),
		favoriteAppIdentities: hasDurableIdentities
			? uniqueStrings(raw.favoriteAppIdentities)
			: [],
		collapsedCategories: uniqueStrings(raw.collapsedCategories).filter(id =>
			known.has(id),
		),
		categoryOverrides: overrides,
		categoryOverrideIdentities: hasDurableIdentities
			? overrideIdentities
			: {},
		hiddenAppIds: uniqueStrings(raw.hiddenAppIds),
		hiddenAppIdentities: hasDurableIdentities
			? uniqueStrings(raw.hiddenAppIdentities)
			: [],
		promotedAppIds: uniqueStrings(raw.promotedAppIds),
		promotedAppIdentities: hasDurableIdentities
			? uniqueStrings(raw.promotedAppIdentities)
			: [],
		legacyCanonicalPreferences,
		...(Object.keys(unknownFields).length > 0 ? { unknownFields } : {}),
	}
}

export function readPreferences(storage: Storage): AppPreferencesV8 {
	return (
		readSlot(storage, PREFERENCES_KEY) ??
		readSlot(storage, PREFERENCES_BACKUP_KEY) ??
		normalizePreferences(null)
	)
}

/** `null` means "nothing usable here", so the caller can fall through to the next source. */
function readSlot(storage: Storage, key: string): AppPreferencesV8 | null {
	try {
		const value = storage.getItem(key)
		return value ? normalizePreferences(JSON.parse(value)) : null
	} catch {
		return null
	}
}

/**
 * Persist preferences, reporting whether the write landed. Storage can refuse silently —
 * quota exhausted, private mode, storage disabled — and the store keeps showing the change
 * either way, so the caller has to know: favorites, hidden apps and custom categories would
 * otherwise disappear at the next start with nothing ever told to the user.
 * Still never throws; a failed write is a reported condition, not an exception.
 */
export function writePreferences(
	storage: Storage,
	preferences: AppPreferencesV8,
): boolean {
	// If the stored document was written by a newer version than this build understands, leave
	// it untouched: overwriting it with the older v5 shape would strip fields the newer build
	// added, silently losing settings the moment the user downgrades. The runtime still reflects
	// the change this session; it simply is not persisted over the newer format. Reported as a
	// success because it is a deliberate protection, not a storage failure.
	if (storedVersionIsNewer(storage)) {
		return true
	}
	rotateBackup(storage)
	try {
		const { unknownFields = {}, ...knownFields } = preferences
		storage.setItem(
			PREFERENCES_KEY,
			JSON.stringify({ ...unknownFields, ...knownFields }),
		)
		return true
	} catch {
		return false
	}
}

function storedVersionIsNewer(storage: Storage): boolean {
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

/**
 * Move the current value aside so the backup always holds the last state that was successfully
 * stored — never the one being written now. Best-effort on purpose: the return value of
 * `writePreferences` answers "were the user's preferences persisted", and a failed backup does
 * not change that answer.
 */
function rotateBackup(storage: Storage): void {
	try {
		const current = storage.getItem(PREFERENCES_KEY)
		if (current) storage.setItem(PREFERENCES_BACKUP_KEY, current)
	} catch {
		/* keeping a spare copy is an improvement, never a precondition */
	}
}
