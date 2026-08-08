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

export const CURRENT_PREFERENCES_VERSION = 13

export const PREFERENCES_BACKUP_KEY = 'windows-apps.preferences.v1.bak'

export interface LegacyCanonicalPreferences {
	favorite: string[]
	hidden: string[]
	promoted: string[]
	installer: string[]
	categoryOverrides: Record<string, AppCategory>
}

export interface AppPreferencesV13 {
	version: 13
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	favoriteAppIds: string[]
	favoriteAppIdentities: string[]
	collapsedCategories: AppCategory[]
	categoryOverrides: Record<string, AppCategory>
	categoryOverrideIdentities: Record<string, AppCategory>
	hiddenAppIds: string[]
	hiddenAppIdentities: string[]
	promotedAppIds: string[]
	promotedAppIdentities: string[]
	installerAppIds: string[]
	installerAppIdentities: string[]
	scenarios: Scenario[]
	favoriteScenarioIds: string[]
	firstSeenAt: Record<string, number>
	legacyCanonicalPreferences: LegacyCanonicalPreferences
	unknownFields?: Record<string, unknown>
}

export const DEFAULT_PREFERENCES: AppPreferencesV13 = {
	version: 13,
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
	favoriteScenarioIds: [],
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
	'favoriteScenarioIds',
	'firstSeenAt',
	'legacyCanonicalPreferences',
])

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

function normalizeTimestampMap(value: unknown): Record<string, number> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return {}
	return Object.fromEntries(
		Object.entries(value as Record<string, unknown>).filter(
			([key, at]) =>
				key.trim() &&
				typeof at === 'number' &&
				Number.isFinite(at) &&
				at > 0,
		),
	) as Record<string, number>
}

export function normalizePreferences(value: unknown): AppPreferencesV13 {
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
	const hasDurableIdentities = version >= 7
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
	const scenarios = hasScenarios ? normalizeScenarios(raw.scenarios) : []
	const unknownFields = Object.fromEntries(
		Object.entries(raw).filter(
			([key]) => !KNOWN_PREFERENCE_FIELDS.has(key),
		),
	)
	return {
		version: 13,
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
		scenarios,
		favoriteScenarioIds: uniqueStrings(raw.favoriteScenarioIds).filter(id =>
			scenarios.some(scenario => scenario.id === id),
		),
		firstSeenAt: hasFirstSeen ? normalizeTimestampMap(raw.firstSeenAt) : {},
		legacyCanonicalPreferences,
		...(Object.keys(unknownFields).length > 0 ? { unknownFields } : {}),
	}
}

export function readPreferences(storage: Storage): AppPreferencesV13 {
	return (
		readSlot(storage, PREFERENCES_KEY) ??
		readSlot(storage, PREFERENCES_BACKUP_KEY) ??
		normalizePreferences(null)
	)
}

function readSlot(storage: Storage, key: string): AppPreferencesV13 | null {
	try {
		const value = storage.getItem(key)
		return value ? normalizePreferences(JSON.parse(value)) : null
	} catch {
		return null
	}
}

export function writePreferences(
	storage: Storage,
	preferences: AppPreferencesV13,
): boolean {
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

function rotateBackup(storage: Storage): void {
	try {
		const current = storage.getItem(PREFERENCES_KEY)
		if (current) storage.setItem(PREFERENCES_BACKUP_KEY, current)
	} catch (ignored) {
		void ignored
	}
}
