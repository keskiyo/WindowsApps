import { useDeferredValue, useMemo } from 'react'
import {
	type AppView,
	type CategorizedAppsState,
	filterVisibleApps,
	isCatalogArtifact,
	rankAppsByQuery,
	selectCatalogCounts,
	selectCategorizedApps,
} from '../../../entities/app'

/** Icons hydrate for the cards a user can plausibly reach without scrolling first. */
const HYDRATION_WINDOW = 48

/**
 * The store fields this view reads. Structural, not `Pick<AppState>`: the widget derives a
 * catalog view, it does not depend on the root store.
 */
interface CatalogViewState extends CategorizedAppsState {
	activeView: AppView
	favoriteAppIds: string[]
	hiddenAppIds: string[]
	query: string
}

/**
 * Everything the shell derives from the catalog, memoized against the store fields each step
 * actually depends on. `App` re-renders on any store change because it subscribes to the whole
 * state, so a derivation left in the component body runs on every keystroke and scan tick.
 */
export function useCatalogView(state: CatalogViewState) {
	// Dedup is O(N) but still recomputed only when the catalog actually changes; query
	// typing, scan progress, favorites and drawer toggles reuse the memoized result.
	const categorizedApps = useMemo(
		() =>
			selectCategorizedApps({
				apps: state.apps,
				categoryOverrides: state.categoryOverrides,
				categoryOverrideIdentities: state.categoryOverrideIdentities,
				promotedAppIds: state.promotedAppIds,
				promotedAppIdentities: state.promotedAppIdentities,
			}),
		[
			state.apps,
			state.categoryOverrides,
			state.categoryOverrideIdentities,
			state.promotedAppIds,
			state.promotedAppIdentities,
		],
	)
	const visibleApps = useMemo(
		() =>
			filterVisibleApps(
				categorizedApps,
				state.activeView,
				state.hiddenAppIds,
				state.favoriteAppIds,
			),
		[
			categorizedApps,
			state.activeView,
			state.hiddenAppIds,
			state.favoriteAppIds,
		],
	)
	const paletteApps = useMemo(() => {
		const hidden = new Set(state.hiddenAppIds)
		return categorizedApps.filter(
			app => !hidden.has(app.id) && !isCatalogArtifact(app),
		)
	}, [categorizedApps, state.hiddenAppIds])
	// Defer the query so fast typing never blocks the input. React will render
	// the grid with the deferred value while keeping the input state current.
	const deferredQuery = useDeferredValue(state.query)
	const filteredApps = useMemo(
		() => rankAppsByQuery(visibleApps, deferredQuery),
		[visibleApps, deferredQuery],
	)
	const counts = useMemo(
		() =>
			selectCatalogCounts(
				categorizedApps,
				state.hiddenAppIds,
				state.favoriteAppIds,
			),
		[categorizedApps, state.hiddenAppIds, state.favoriteAppIds],
	)
	// A joined string, not the array: the hydration effect must fire when the *set* of visible
	// ids changes, and a fresh array of the same ids is a new reference on every render.
	const visibleHydrationIds = useMemo(
		() =>
			filteredApps
				.slice(0, HYDRATION_WINDOW)
				.map(app => app.id)
				.join('|'),
		[filteredApps],
	)

	return {
		counts,
		deferredQuery,
		filteredApps,
		paletteApps,
		visibleHydrationIds,
	}
}
