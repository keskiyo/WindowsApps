import type {
	AppCategory,
	CustomCategoryAccent,
} from '../../../../entities/category'

type CategoryAccent =
	| 'yellow'
	| 'cyan'
	| 'pink'
	| 'green'
	| 'blue'
	| 'orange'
	| 'purple'
	| 'red'
	| 'slate'
	| 'neutral'

export const navigationCategoryRowClass =
	'navigation-category-row relative flex w-full items-center rounded-lg border border-(--border-neutral) border-l-2 bg-(--surface-panel) px-3 py-2 text-left text-sm text-(--text-primary)'

export const navigationCategoryCountClass =
	'navigation-category-count ml-auto shrink-0 rounded-md border px-1.5 py-0.5 text-xs'

const BUILT_IN_CATEGORY_ACCENTS: Record<string, CategoryAccent> = {
	games: 'yellow',
	ai: 'cyan',
	editors: 'pink',
	development: 'blue',
	productivity: 'green',
	browsers: 'purple',
	media: 'red',
	communication: 'orange',
	file_cloud: 'cyan',
	security: 'green',
	utilities: 'purple',
	system: 'slate',
	windows_features: 'slate',
	other: 'purple',
}

export function categoryAccent(
	category: AppCategory,
	customAccent?: CustomCategoryAccent,
) {
	return BUILT_IN_CATEGORY_ACCENTS[category] ?? customAccent ?? 'neutral'
}
