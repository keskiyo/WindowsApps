import { useDeferredValue, useMemo } from 'react'
import {
	appIdentity,
	type AppView,
	type CategorizedAppsState,
	createMarkLookup,
	filterVisibleApps,
	isCatalogArtifact,
	rankAppsByQueryAndCategory,
	selectCatalogCounts,
	selectCategorizedApps,
	selectRecentApps,
	selectSearchScopeCounts,
} from '../../../entities/app'
import type { CategoryDefinition } from '../../../entities/category'
import { resolveScenarioApps, type Scenario } from '../../../entities/scenario'
import { buildMorePreview } from '../lib/morePreview'

export type { MorePreview, MorePreviewItem } from '../lib/morePreview'

const HYDRATION_WINDOW = 48
const PALETTE_SUGGESTIONS = 12

interface CatalogViewState extends CategorizedAppsState {
	activeView: AppView
	categories: CategoryDefinition[]
	favoriteAppIds: string[]
	favoriteAppIdentities: string[]
	firstSeenAt: Record<string, number>
	hiddenAppIds: string[]
	hiddenAppIdentities: string[]
	query: string
	scenarios: Scenario[]
}

export function useCatalogView(state: CatalogViewState) {
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
	const isHidden = useMemo(
		() => createMarkLookup(state.hiddenAppIds, state.hiddenAppIdentities),
		[state.hiddenAppIds, state.hiddenAppIdentities],
	)
	const isFavorite = useMemo(
		() =>
			createMarkLookup(state.favoriteAppIds, state.favoriteAppIdentities),
		[state.favoriteAppIds, state.favoriteAppIdentities],
	)
	const visibleApps = useMemo(
		() =>
			filterVisibleApps(
				categorizedApps,
				state.activeView,
				isHidden,
				isFavorite,
			),
		[categorizedApps, state.activeView, isHidden, isFavorite],
	)
	const catalogApps = useMemo(
		() =>
			categorizedApps.filter(
				app => !isHidden(app) && !isCatalogArtifact(app),
			),
		[categorizedApps, isHidden],
	)
	const primaryApps = useMemo(
		() => catalogApps.filter(app => app.visibilityClass !== 'auxiliary'),
		[catalogApps],
	)
	const paletteSuggestions = useMemo(() => {
		const favorites = new Set(state.favoriteAppIds)
		const starred = primaryApps.filter(app => favorites.has(app.id))
		const rest = primaryApps.filter(app => !favorites.has(app.id))
		return [
			...starred,
			...selectRecentApps(
				rest,
				app => state.firstSeenAt[appIdentity(app)] ?? 0,
				PALETTE_SUGGESTIONS,
			),
		].slice(0, PALETTE_SUGGESTIONS)
	}, [primaryApps, state.favoriteAppIds, state.firstSeenAt])
	const deferredQuery = useDeferredValue(state.query)
	const filteredApps = useMemo(
		() =>
			rankAppsByQueryAndCategory(
				visibleApps,
				deferredQuery,
				state.categories,
			),
		[visibleApps, deferredQuery, state.categories],
	)
	const counts = useMemo(
		() => selectCatalogCounts(categorizedApps, isHidden, isFavorite),
		[categorizedApps, isHidden, isFavorite],
	)
	const searchScopeCounts = useMemo(
		() =>
			selectSearchScopeCounts(
				categorizedApps,
				deferredQuery,
				state.hiddenAppIds,
			),
		[categorizedApps, deferredQuery, state.hiddenAppIds],
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
		primaryApps,
		paletteSuggestions,
		searchScopeCounts,
		visibleHydrationIds,
	}
}
