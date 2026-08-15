import { useMemo } from 'react'
import { type AppInfo, rankAppsByQueryAndCategory } from '../../../../entities/app'
import type { CategoryDefinition } from '../../../../entities/category'

const BY_NAME = new Intl.Collator(undefined, {
	sensitivity: 'base',
	numeric: true,
})

export function usePickerResults(
	apps: AppInfo[],
	categories: CategoryDefinition[],
	query: string,
): AppInfo[] {
	const byName = useMemo(
		() => [...apps].sort((left, right) => BY_NAME.compare(left.name, right.name)),
		[apps],
	)

	return useMemo(
		() => rankAppsByQueryAndCategory(byName, query, categories),
		[byName, categories, query],
	)
}
