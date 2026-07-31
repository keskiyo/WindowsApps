import type { AppCategory, CustomCategoryAccent } from '../../../types'

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
