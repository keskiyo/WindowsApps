import {
	type AppCategory,
	type CategoryDefinition,
	DEFAULT_CATEGORIES,
	isCustomCategoryAccent,
	stableCustomCategoryAccent,
} from '../../entities/category'

export function uniqueStrings(value: unknown): string[] {
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

export function normalizeOverrideMap(
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

export function normalizeDefinitions(value: unknown): CategoryDefinition[] {
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

export function normalizeTimestampMap(value: unknown): Record<string, number> {
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
