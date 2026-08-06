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
import { resolveScenarioApps, type Scenario } from '../../../entities/scenario'
import { buildMorePreview } from '../lib/morePreview'

export type { MorePreview, MorePreviewItem } from '../lib/morePreview'

/** Icons hydrate for the cards a user can plausibly reach without scrolling first. */
const HYDRATION_WINDOW = 48

/**
 * The store fields this view reads. Structural, not `Pick<AppState>`: the widget derives a
 * catalog view, it does not depend on the root store.
 */
interface CatalogViewState extends CategorizedAppsState {
	activeView: AppView
	favoriteAppIds: string[]
	firstSeenAt: Record<string, number>
	hiddenAppIds: string[]
	query: string
	scenarios: Scenario[]
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
				installerAppIds: state.installerAppIds,
				installerAppIdentities: state.installerAppIdentities,
			}),
		[
			state.apps,
			state.categoryOverrides,
			state.categoryOverrideIdentities,
			state.promotedAppIds,
			state.promotedAppIdentities,
			state.installerAppIds,
			state.installerAppIdentities,
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
	/** Everything a scenario may reference, auxiliary tools included: it stores what the user picked. */
	const catalogApps = useMemo(() => {
		const hidden = new Set(state.hiddenAppIds)
		return categorizedApps.filter(
			app => !hidden.has(app.id) && !isCatalogArtifact(app),
		)
	}, [categorizedApps, state.hiddenAppIds])
	/**
	 * Quick launch offers what the grid shows. Auxiliary entries are updater stubs, command
	 * environments and product components — Discord's Squirrel stub sat next to Discord itself under
	 * the same name and icon, so the palette asked the user to choose between two identical rows.
	 */
	const paletteApps = useMemo(
		() => catalogApps.filter(app => app.visibilityClass !== 'auxiliary'),
		[catalogApps],
	)
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
	const morePreview = useMemo(
		() =>
			buildMorePreview({
				categorizedApps,
				favoriteAppIds: state.favoriteAppIds,
				firstSeenAt: state.firstSeenAt,
				hiddenAppIds: state.hiddenAppIds,
				scenarios: state.scenarios,
			}),
		[
			categorizedApps,
			state.favoriteAppIds,
			state.firstSeenAt,
			state.hiddenAppIds,
			state.scenarios,
		],
	)
	// Both pages show scenario apps by their icon — the scenarios page in its lists, More in the
	// run dialog — and neither renders a grid, so without this the tiles would sit on the fallback
	// glyph until the user happened to visit a catalog view first.
	const scenarioApps = useMemo(() => {
		if (state.activeView !== 'scenarios' && state.activeView !== 'more')
			return []
		const identities = new Set(
			state.scenarios.flatMap(scenario => [
				...scenario.launchIdentities,
				...scenario.closeIdentities,
			]),
		)
		return resolveScenarioApps([...identities], catalogApps).apps
	}, [catalogApps, state.activeView, state.scenarios])
	/** The apps the active view actually puts on screen, whether or not it renders a grid. */
	const hydrationApps = useMemo(() => {
		if (state.activeView === 'settings') return []
		if (state.activeView === 'scenarios') return scenarioApps
		if (state.activeView === 'more')
			return [
				...morePreview.auxiliary,
				...morePreview.hidden,
				...morePreview.installersDocs,
			]
				.map(entry => entry.app)
				.concat(scenarioApps)
		return filteredApps
	}, [filteredApps, morePreview, scenarioApps, state.activeView])
	// A joined string, not the array: the hydration effect must fire when the *set* of visible
	// ids changes, and a fresh array of the same ids is a new reference on every render.
	const visibleHydrationIds = useMemo(
		() =>
			[...new Set(hydrationApps.map(app => app.id))]
				.slice(0, HYDRATION_WINDOW)
				.join('|'),
		[hydrationApps],
	)

	return {
		catalogApps,
		counts,
		deferredQuery,
		filteredApps,
		morePreview,
		paletteApps,
		visibleHydrationIds,
	}
}
