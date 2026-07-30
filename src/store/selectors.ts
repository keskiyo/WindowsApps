import { deduplicateVisibleApps } from '../lib/appDeduplication'
import {
	filterAppsByQuery,
	rankAppsByQuery,
} from '../lib/catalogSearch'
import type { AppInfo, AppView } from '../types'
import type { AppState } from './appStore'

export { filterAppsByQuery, rankAppsByQuery }

export function filterVisibleApps(
	categorized: AppInfo[],
	activeView: AppView,
	hiddenAppIds: string[],
	favoriteAppIds: string[],
): AppInfo[] {
	if (activeView === 'settings') return []
	// Set lookups, not Array.includes: this runs over the whole catalog on every
	// hydration patch, so a linear scan per app makes it O(apps × hidden).
	const hidden = new Set(hiddenAppIds)
	if (activeView === 'auxiliary')
		return categorized.filter(
			app => app.visibilityClass === 'auxiliary' && !hidden.has(app.id),
		)
	if (activeView === 'hidden')
		return categorized.filter(app => hidden.has(app.id))
	const visible = categorized.filter(
		app => app.visibilityClass !== 'auxiliary' && !hidden.has(app.id),
	)
	if (activeView !== 'favorites') return visible
	const favorites = new Set(favoriteAppIds)
	return visible.filter(app => favorites.has(app.id))
}

type CategorizedAppsState = Pick<
	AppState,
	| 'apps'
	| 'categoryOverrides'
	| 'categoryOverrideIdentities'
	| 'promotedAppIds'
	| 'promotedAppIdentities'
>

/**
 * Object identity is the point here, not just speed. Every `catalog://patches` event
 * replaces the apps array while icons stream in, and cloning every record would hand each
 * card a new `app` reference, defeating `memo(AppCard)` and re-rendering the whole grid
 * ~N/24 times during startup. Records that nothing applies to are returned untouched, so a
 * patch only invalidates the cards it actually changed.
 */
export function selectCategorizedApps(state: CategorizedAppsState): AppInfo[] {
	const promotedIds = new Set(state.promotedAppIds)
	const promotedIdentities = new Set(state.promotedAppIdentities)
	return deduplicateVisibleApps(
		state.apps.map(app => {
			// Identity-first: the durable override (keyed by canonicalIdentity) wins so a manual
			// category survives a Force full scan / Reset cache / dedup change that renamed the id.
			const category =
				state.categoryOverrideIdentities[
					app.preferenceIdentity ??
						app.canonicalIdentity ??
						app.id
				] ??
				state.categoryOverrides[app.id] ??
				app.category
			const promote =
				app.visibilityClass === 'auxiliary' &&
				(promotedIds.has(app.id) ||
					promotedIdentities.has(
						app.preferenceIdentity ??
							app.canonicalIdentity ??
							app.id,
					))
			if (category === app.category && !promote) return app
			const categorized = { ...app, category }
			return promote
				? {
						...categorized,
						visibilityClass: 'primary' as const,
						userPromoted: true,
					}
				: categorized
		}),
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
