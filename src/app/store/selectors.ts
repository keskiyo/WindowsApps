import {
	type AppInfo,
	filterAppsByQuery,
	filterVisibleApps,
	selectCategorizedApps,
} from '../../entities/app'
import type { AppState } from './types'

export {
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
		state.hiddenAppIds,
		state.favoriteAppIds,
	)
}

export function selectFilteredApps(state: AppState): AppInfo[] {
	return filterAppsByQuery(selectVisibleApps(state), state.query)
}
