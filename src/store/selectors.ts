import { deduplicateVisibleApps } from '../lib/appDeduplication'
import type { AppInfo, AppView } from '../types'
import type { AppState } from './appStore'

export function filterVisibleApps(
	categorized: AppInfo[],
	activeView: AppView,
	hiddenAppIds: string[],
	favoriteAppIds: string[],
): AppInfo[] {
	if (activeView === 'settings') return []
	if (activeView === 'hidden')
		return categorized.filter(app => hiddenAppIds.includes(app.id))
	const visible = categorized.filter(app => !hiddenAppIds.includes(app.id))
	return activeView === 'favorites'
		? visible.filter(app => favoriteAppIds.includes(app.id))
		: visible
}

export function filterAppsByQuery(apps: AppInfo[], query: string): AppInfo[] {
	const normalized = query.trim().toLocaleLowerCase()
	if (!normalized) return apps
	return apps.filter(app =>
		[app.name, app.publisher, app.description].some(value =>
			value?.toLocaleLowerCase().includes(normalized),
		),
	)
}

export function selectCategorizedApps(state: AppState): AppInfo[] {
	return deduplicateVisibleApps(
		state.apps.map(app => ({
			...app,
			category: state.categoryOverrides[app.id] ?? app.category,
		})),
	)
}

export function selectVisibleApps(state: AppState): AppInfo[] {
	return filterVisibleApps(
		selectCategorizedApps(state),
		state.activeView,
		state.hiddenAppIds,
		state.favoriteAppIds,
	)
}

export function selectFilteredApps(state: AppState): AppInfo[] {
	return filterAppsByQuery(selectVisibleApps(state), state.query)
}
