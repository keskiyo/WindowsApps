import {
	type AppInfo,
	createMarkLookup,
	filterAppsByQuery,
	filterVisibleApps,
	selectCategorizedApps,
} from '../../entities/app'
import type { AppState } from './types'

export {
	createMarkLookup,
	filterAppsByQuery,
	filterVisibleApps,
	rankAppsByQuery,
	rankAppsByQueryTop,
	selectCatalogCounts,
	selectCategorizedApps,
	type CatalogCounts,
} from '../../entities/app'

export function selectVisibleApps(state: AppState): AppInfo[] {
	return filterVisibleApps(
		selectCategorizedApps(state),
		state.activeView,
		createMarkLookup(state.hiddenAppIds, state.hiddenAppIdentities),
		createMarkLookup(state.favoriteAppIds, state.favoriteAppIdentities),
	)
}

export function selectFilteredApps(state: AppState): AppInfo[] {
	return filterAppsByQuery(selectVisibleApps(state), state.query)
}
