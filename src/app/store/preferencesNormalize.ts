import {
	normalizeDefinitions,
	normalizeOverrideMap,
	normalizeTimestampMap,
	uniqueStrings,
} from './preferencesFields'
import { normalizeScenarios } from './preferencesScenarios'
import {
	type AppPreferencesV16,
	DEFAULT_PREFERENCES,
	type LegacyCanonicalPreferences,
} from './preferencesSchema'

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
	'documentAppIds',
	'documentAppIdentities',
	'scenarios',
	'favoriteScenarioIds',
	'firstSeenAt',
	'legacyCanonicalPreferences',
])

function readLegacy(
	raw: Record<string, unknown>,
	hasDurableIdentities: boolean,
	hasInstallerMarks: boolean,
	hasDocumentMarks: boolean,
	known: Set<string>,
): LegacyCanonicalPreferences {
	const stored =
		hasDurableIdentities &&
		raw.legacyCanonicalPreferences &&
		typeof raw.legacyCanonicalPreferences === 'object' &&
		!Array.isArray(raw.legacyCanonicalPreferences)
			? (raw.legacyCanonicalPreferences as Record<string, unknown>)
			: {}
	return {
		favorite: uniqueStrings(
			hasDurableIdentities ? stored.favorite : raw.favoriteAppIdentities,
		),
		hidden: uniqueStrings(
			hasDurableIdentities ? stored.hidden : raw.hiddenAppIdentities,
		),
		promoted: uniqueStrings(
			hasDurableIdentities ? stored.promoted : raw.promotedAppIdentities,
		),
		installer: uniqueStrings(hasInstallerMarks ? stored.installer : []),
		document: uniqueStrings(hasDocumentMarks ? stored.document : []),
		categoryOverrides: normalizeOverrideMap(
			hasDurableIdentities
				? stored.categoryOverrides
				: raw.categoryOverrideIdentities,
			known,
		),
	}
}

export function normalizePreferences(value: unknown): AppPreferencesV16 {
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
	const version = typeof raw.version === 'number' ? raw.version : 0
	const hasDurableIdentities = version >= 7
	const hasInstallerMarks = version >= 9
	const hasDocumentMarks = version >= 16
	const hasFirstSeen = version >= 10
	const hasScenarios = version >= 11
	const scenarios = hasScenarios ? normalizeScenarios(raw.scenarios) : []
	const unknownFields = Object.fromEntries(
		Object.entries(raw).filter(
			([key]) => !KNOWN_PREFERENCE_FIELDS.has(key),
		),
	)
	return {
		version: 16,
		categories,
		categoryOrder,
		favoriteAppIds: uniqueStrings(raw.favoriteAppIds),
		favoriteAppIdentities: hasDurableIdentities
			? uniqueStrings(raw.favoriteAppIdentities)
			: [],
		collapsedCategories: uniqueStrings(raw.collapsedCategories).filter(id =>
			known.has(id),
		),
		categoryOverrides: normalizeOverrideMap(raw.categoryOverrides, known),
		categoryOverrideIdentities: hasDurableIdentities
			? normalizeOverrideMap(raw.categoryOverrideIdentities, known)
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
		documentAppIds: hasDocumentMarks
			? uniqueStrings(raw.documentAppIds)
			: [],
		documentAppIdentities: hasDocumentMarks
			? uniqueStrings(raw.documentAppIdentities)
			: [],
		scenarios,
		favoriteScenarioIds: uniqueStrings(raw.favoriteScenarioIds).filter(id =>
			scenarios.some(scenario => scenario.id === id),
		),
		firstSeenAt: hasFirstSeen ? normalizeTimestampMap(raw.firstSeenAt) : {},
		legacyCanonicalPreferences: readLegacy(
			raw,
			hasDurableIdentities,
			hasInstallerMarks,
			hasDocumentMarks,
			known,
		),
		...(Object.keys(unknownFields).length > 0 ? { unknownFields } : {}),
	}
}
