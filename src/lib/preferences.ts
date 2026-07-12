import {
	CATEGORY_ORDER,
	DEFAULT_CATEGORIES,
	type AppCategory,
	type CategoryDefinition,
} from '../types'

export const PREFERENCES_KEY = 'windows-apps.preferences.v1'

export interface AppPreferencesV4 {
	version: 4
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	favoriteAppIds: string[]
	collapsedCategories: AppCategory[]
	categoryOverrides: Record<string, AppCategory>
	hiddenAppIds: string[]
	promotedAppIds: string[]
	promotedAppIdentities: string[]
}

export const DEFAULT_PREFERENCES: AppPreferencesV4 = {
	version: 4,
	categories: DEFAULT_CATEGORIES.map(category => ({ ...category })),
	categoryOrder: [...CATEGORY_ORDER],
	favoriteAppIds: [],
	collapsedCategories: [],
	categoryOverrides: {},
	hiddenAppIds: [],
	promotedAppIds: [],
	promotedAppIdentities: [],
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
		categories.push({ id, label, builtIn: false })
		labels.add(label.toLocaleLowerCase())
	}
	return categories
}

export function normalizePreferences(value: unknown): AppPreferencesV4 {
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
	const overrides =
		raw.categoryOverrides &&
		typeof raw.categoryOverrides === 'object' &&
		!Array.isArray(raw.categoryOverrides)
			? Object.fromEntries(
					Object.entries(raw.categoryOverrides).filter(
						([id, category]) =>
							id.trim() &&
							typeof category === 'string' &&
							known.has(category),
					),
				)
			: {}
	return {
		version: 4,
		categories,
		categoryOrder,
		favoriteAppIds: uniqueStrings(raw.favoriteAppIds),
		collapsedCategories: uniqueStrings(raw.collapsedCategories).filter(id =>
			known.has(id),
		),
		categoryOverrides: overrides,
		hiddenAppIds: uniqueStrings(raw.hiddenAppIds),
		promotedAppIds: uniqueStrings(raw.promotedAppIds),
		promotedAppIdentities: uniqueStrings(raw.promotedAppIdentities),
	}
}

export function readPreferences(storage: Storage): AppPreferencesV4 {
	try {
		const value = storage.getItem(PREFERENCES_KEY)
		return value
			? normalizePreferences(JSON.parse(value))
			: normalizePreferences(null)
	} catch {
		return normalizePreferences(null)
	}
}

export function writePreferences(
	storage: Storage,
	preferences: AppPreferencesV4,
): void {
	try {
		storage.setItem(PREFERENCES_KEY, JSON.stringify(preferences))
	} catch {
		/* optional */
	}
}
