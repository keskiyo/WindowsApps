import type { AppInfo } from '../model/app.types'
import type { AppCategory } from '../../category'

export type DragData = {
	type?: string
	appId?: string
	category?: AppCategory
}

export type DropAction =
	| { type: 'move-app'; appId: string; category: AppCategory }
	| { type: 'reorder-category'; active: AppCategory; over: AppCategory }

export function groupAppsByCategory(
	apps: readonly AppInfo[],
): Map<AppCategory, AppInfo[]> {
	const groups = new Map<AppCategory, AppInfo[]>()
	for (const app of apps) {
		const group = groups.get(app.category)
		if (group) group.push(app)
		else groups.set(app.category, [app])
	}
	return groups
}

export function sortFavoritesFirst(
	apps: readonly AppInfo[],
	favoriteAppIds: readonly string[],
): AppInfo[] {
	const isFavorite = new Set(favoriteAppIds)
	return [...apps].sort((a, b) => {
		const aFav = isFavorite.has(a.id) ? 0 : 1
		const bFav = isFavorite.has(b.id) ? 0 : 1
		return (
			aFav - bFav ||
			a.name.localeCompare(b.name, undefined, { sensitivity: 'base' })
		)
	})
}

export function getDropAction(
	active: DragData | undefined,
	over: DragData | undefined,
): DropAction | null {
	if (
		active?.type === 'app' &&
		active.appId &&
		over?.category &&
		(over.type === 'category' || over.type === 'category-sort')
	)
		return {
			type: 'move-app',
			appId: active.appId,
			category: over.category,
		}
	if (
		active?.type === 'category-sort' &&
		active.category &&
		(over?.type === 'category-sort' || over?.type === 'category') &&
		over.category &&
		active.category !== over.category
	)
		return {
			type: 'reorder-category',
			active: active.category,
			over: over.category,
		}
	return null
}
