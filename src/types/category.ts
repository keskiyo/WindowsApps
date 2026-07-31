export const CATEGORY_ORDER = [
	'games',
	'ai',
	'editors',
	'development',
	'productivity',
	'browsers',
	'media',
	'communication',
	'file_cloud',
	'security',
	'utilities',
	'system',
	'windows_features',
	'other',
] as const
export type BuiltInCategory = (typeof CATEGORY_ORDER)[number]
export type AppCategory = string
export type CustomCategoryAccent =
	| 'yellow'
	| 'cyan'
	| 'pink'
	| 'green'
	| 'blue'
	| 'orange'
	| 'purple'
	| 'red'

export interface CategoryDefinition {
	id: AppCategory
	label: string
	builtIn: boolean
	accent?: CustomCategoryAccent
}

export const CATEGORY_LABELS: Record<string, string> = {
	games: 'Games',
	ai: 'AI & Agents',
	editors: 'Editors & Design',
	development: 'Development',
	productivity: 'Office & Productivity',
	browsers: 'Browsers',
	media: 'Media',
	communication: 'Communication',
	file_cloud: 'File & Cloud',
	security: 'Security & Privacy',
	utilities: 'Utilities',
	system: 'System',
	windows_features: 'Windows Features',
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
