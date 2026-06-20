export const CATEGORY_ORDER = [
	'games',
	'ai',
	'editors',
	'development',
	'browsers',
	'media',
	'communication',
	'utilities',
	'system',
	'other',
] as const
export type BuiltInCategory = (typeof CATEGORY_ORDER)[number]
export type AppCategory = string

export interface CategoryDefinition {
	id: AppCategory
	label: string
	builtIn: boolean
}

export const CATEGORY_LABELS: Record<string, string> = {
	games: 'Games',
	ai: 'AI & Agents',
	editors: 'Editors & Design',
	development: 'Development',
	browsers: 'Browsers',
	media: 'Media',
	communication: 'Communication',
	utilities: 'Utilities',
	system: 'System',
	other: 'Other',
}

export const DEFAULT_CATEGORIES: CategoryDefinition[] = CATEGORY_ORDER.map(
	id => ({
		id,
		label: CATEGORY_LABELS[id]!,
		builtIn: true,
	}),
)

export function categoryLabel(
	categories: CategoryDefinition[],
	id: AppCategory,
): string {
	return categories.find(category => category.id === id)?.label ?? id
}
