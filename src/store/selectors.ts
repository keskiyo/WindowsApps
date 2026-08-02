import { deduplicateVisibleApps } from '../lib/appDeduplication'
import {
	filterAppsByQuery,
	rankAppsByQuery,
	rankAppsByQueryTop,
} from '../lib/catalogSearch'
import type { AppInfo, AppView } from '../types'
import {
	INSTALLERS_DOCS_CATEGORY,
	isCatalogArtifact,
} from '../lib/catalogArtifacts'
import type { AppState } from './appStore'

export { filterAppsByQuery, rankAppsByQuery, rankAppsByQueryTop }

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
	if (activeView === 'installers_docs')
		return categorized.filter(
			app => isCatalogArtifact(app) && !hidden.has(app.id),
		)
	if (activeView === 'auxiliary')
		return categorized.filter(
			app =>
				!isCatalogArtifact(app) &&
				app.visibilityClass === 'auxiliary' &&
				!hidden.has(app.id),
		)
	if (activeView === 'hidden')
		return categorized.filter(app => hidden.has(app.id))
	const visible = categorized.filter(
		app =>
			!isCatalogArtifact(app) &&
			app.visibilityClass !== 'auxiliary' &&
			!hidden.has(app.id),
	)
	if (activeView !== 'favorites') return visible
	const favorites = new Set(favoriteAppIds)
	return visible.filter(app => favorites.has(app.id))
}

export interface CatalogCounts {
	/** Non-auxiliary, non-hidden apps: the set the grid, the header and the drawer list show. */
	visibleCategorizedApps: AppInfo[]
	/** Per-category counts over `visibleCategorizedApps`. */
	navigationCounts: Map<string, number>
	/** Each badge is the length of the view it opens, so the number never contradicts the list. */
	favoriteCount: number
	hiddenCount: number
	auxiliaryCount: number
	/** Scanner classification totals for the settings page; hidden apps still count as scanned. */
	classifiedPrimaryCount: number
	classifiedAuxiliaryCount: number
}

/**
 * Every count the navigation and settings surfaces show, derived in one `Set`-based pass.
 *
 * It exists as a selector for two reasons. It is the single definition the sidebar and the
 * drawer share — they used to disagree, one counting favorites over the whole categorized
 * catalog and the other over the visible subset, so the same nav item showed different numbers
 * at different window widths. And `App` subscribes to the entire store, so the nested
 * `Array.includes`/`Array.some` scans this replaces re-ran over the catalog on every keystroke.
 *
 * The badges deliberately mirror `filterVisibleApps`: a count that disagrees with the list it
 * opens is a bug, not a different metric. The two `classified*` totals are the exception, and
 * are named for it — the settings page reports what the scanner classified, hidden or not.
 */
export function selectCatalogCounts(
	categorized: AppInfo[],
	hiddenAppIds: string[],
	favoriteAppIds: string[],
): CatalogCounts {
	const hidden = new Set(hiddenAppIds)
	const favorites = new Set(favoriteAppIds)
	const visibleCategorizedApps: AppInfo[] = []
	const navigationCounts = new Map<string, number>()
	let favoriteCount = 0
	let hiddenCount = 0
	let auxiliaryCount = 0
	let classifiedAuxiliaryCount = 0
	for (const app of categorized) {
		const isHidden = hidden.has(app.id)
		if (isHidden) hiddenCount += 1
		if (isCatalogArtifact(app)) {
			if (!isHidden)
				navigationCounts.set(
					INSTALLERS_DOCS_CATEGORY,
					(navigationCounts.get(INSTALLERS_DOCS_CATEGORY) ?? 0) + 1,
				)
			continue
		}
		if (app.visibilityClass === 'auxiliary') {
			classifiedAuxiliaryCount += 1
			if (!isHidden) auxiliaryCount += 1
			continue
		}
		if (isHidden) continue
		visibleCategorizedApps.push(app)
		navigationCounts.set(
			app.category,
			(navigationCounts.get(app.category) ?? 0) + 1,
		)
		if (favorites.has(app.id)) favoriteCount += 1
	}
	return {
		visibleCategorizedApps,
		navigationCounts,
		favoriteCount,
		hiddenCount,
		auxiliaryCount,
		classifiedPrimaryCount: categorized.length - classifiedAuxiliaryCount,
		classifiedAuxiliaryCount,
	}
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
			if (isCatalogArtifact(app)) {
				return app.category === INSTALLERS_DOCS_CATEGORY
					? app
					: { ...app, category: INSTALLERS_DOCS_CATEGORY }
			}
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
			const safeCategory =
				category === INSTALLERS_DOCS_CATEGORY ? app.category : category
			const promote =
				app.visibilityClass === 'auxiliary' &&
				(promotedIds.has(app.id) ||
					promotedIdentities.has(
						app.preferenceIdentity ??
							app.canonicalIdentity ??
							app.id,
					))
			if (safeCategory === app.category && !promote) return app
			const categorized = { ...app, category: safeCategory }
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
