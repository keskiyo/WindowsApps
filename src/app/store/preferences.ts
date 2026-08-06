import {
	type AppCategory,
	CATEGORY_ORDER,
	type CategoryDefinition,
	DEFAULT_CATEGORIES,
	isCustomCategoryAccent,
	stableCustomCategoryAccent,
} from '../../entities/category'
import {
	MAX_SCENARIO_ENTRIES,
	MAX_SCENARIOS,
	type Scenario,
} from '../../entities/scenario'

export const PREFERENCES_KEY = 'windows-apps.preferences.v1'

/** The schema version this build understands. `version` in the stored document is independent
 * of the key name; see `AGENTS_frontend.md` §3. */
export const CURRENT_PREFERENCES_VERSION = 12

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
	installer: string[]
	categoryOverrides: Record<string, AppCategory>
}

export interface AppPreferencesV12 {
	version: 12
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
	// Applications the user filed into Installers & Docs by hand. The scanner's own verdict lives
	// on `AppInfo.artifactKind` and is not stored here: only the manual marks are user data.
	// There is no documentation counterpart on purpose — the scanner detects docs reliably, and a
	// manual mark always means "this is an installer".
	installerAppIds: string[]
	installerAppIdentities: string[]
	// Named launch/close lists. Their entries are card identities for the same reason the sets
	// above are: a scenario keyed by catalog id would quietly empty itself after a Force full scan.
	scenarios: Scenario[]
	// When each card was first seen in the catalog, keyed by the durable card identity. The
	// catalog itself carries no timestamp, so this is the only source for "recently added"; it is
	// pruned to the current catalog on every load, which bounds it to the catalog's size.
	firstSeenAt: Record<string, number>
	legacyCanonicalPreferences: LegacyCanonicalPreferences
	unknownFields?: Record<string, unknown>
}

export const DEFAULT_PREFERENCES: AppPreferencesV12 = {
	version: 12,
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
	installerAppIds: [],
	installerAppIdentities: [],
	scenarios: [],
	firstSeenAt: {},
	legacyCanonicalPreferences: {
		favorite: [],
		hidden: [],
		promoted: [],
		installer: [],
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
	'installerAppIds',
	'installerAppIdentities',
	'scenarios',
	'firstSeenAt',
	'legacyCanonicalPreferences',
])

/**
 * Scenarios as the store may use them: named, bounded, and free of entries it cannot act on.
 * A malformed record is dropped rather than repaired — a scenario with half its list missing
 * would run something the user never chose.
 */
function normalizeScenarios(value: unknown): Scenario[] {
	if (!Array.isArray(value)) return []
	const seenIds = new Set<string>()
	const scenarios: Scenario[] = []
	for (const item of value.slice(0, MAX_SCENARIOS)) {
		if (!item || typeof item !== 'object' || Array.isArray(item)) continue
		const raw = item as Record<string, unknown>
		const id = typeof raw.id === 'string' ? raw.id.trim() : ''
		const name = typeof raw.name === 'string' ? raw.name.trim() : ''
		if (!id || !name || seenIds.has(id)) continue
		seenIds.add(id)
		// v11 scenarios carry no creation date. Stamping them with the migration's own clock
		// would say "created today" about a scenario made weeks ago, so they stay undated.
		const createdAt = raw.createdAt
		scenarios.push({
			id,
			name,
			launchIdentities: uniqueStrings(raw.launchIdentities).slice(
				0,
				MAX_SCENARIO_ENTRIES,
			),
			closeIdentities: uniqueStrings(raw.closeIdentities).slice(
				0,
				MAX_SCENARIO_ENTRIES,
			),
			createdAt:
				typeof createdAt === 'number' &&
				Number.isFinite(createdAt) &&
				createdAt > 0
					? createdAt
					: null,
		})
	}
	return scenarios
}

/** A `{ identity -> epoch millis }` map, keeping only usable keys and finite positive times. */
function normalizeTimestampMap(value: unknown): Record<string, number> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return {}
	return Object.fromEntries(
		Object.entries(value as Record<string, unknown>).filter(
			([key, at]) =>
				key.trim() && typeof at === 'number' && Number.isFinite(at) && at > 0,
		),
	) as Record<string, number>
}

export function normalizePreferences(value: unknown): AppPreferencesV12 {
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
	const version = typeof raw.version === 'number' ? raw.version : 0
	// Durable identities landed in v7 and every later version keeps them; a `>=` test upgrades a
	// document written by a newer build without wiping the identities it does carry.
	const hasDurableIdentities = version >= 7
	// v9 added the manual installer marks, v10 the first-seen stamps, v11 the scenarios. Nothing
	// older can carry them, so an earlier document upgrades to an empty value rather than to a
	// guess — and the stamps refill themselves from the next catalog load.
	const hasInstallerMarks = version >= 9
	const hasFirstSeen = version >= 10
	const hasScenarios = version >= 11
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
		installer: uniqueStrings(hasInstallerMarks ? rawLegacy.installer : []),
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
		version: 12,
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
		installerAppIds: hasInstallerMarks
			? uniqueStrings(raw.installerAppIds)
			: [],
		installerAppIdentities: hasInstallerMarks
			? uniqueStrings(raw.installerAppIdentities)
			: [],
		scenarios: hasScenarios ? normalizeScenarios(raw.scenarios) : [],
		firstSeenAt: hasFirstSeen ? normalizeTimestampMap(raw.firstSeenAt) : {},
		legacyCanonicalPreferences,
		...(Object.keys(unknownFields).length > 0 ? { unknownFields } : {}),
	}
}

export function readPreferences(storage: Storage): AppPreferencesV12 {
	return (
		readSlot(storage, PREFERENCES_KEY) ??
		readSlot(storage, PREFERENCES_BACKUP_KEY) ??
		normalizePreferences(null)
	)
}

/** `null` means "nothing usable here", so the caller can fall through to the next source. */
function readSlot(storage: Storage, key: string): AppPreferencesV12 | null {
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
	preferences: AppPreferencesV12,
): boolean {
	// If the stored document was written by a newer version than this build understands, leave
	// it untouched: overwriting it with the older shape would strip fields the newer build
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
