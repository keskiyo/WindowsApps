import {
	type AppInfo,
	filterAppsByQuery,
	filterVisibleApps,
	selectCategorizedApps,
} from '../../entities/app'
import type { AppState } from './appStore'

// The catalog derivations themselves belong to the App entity, so every layer can reuse them
// without reaching up into the root store. Only the two store-shaped selectors below live here.
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
