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

const HYDRATION_WINDOW = 48

interface CatalogViewState extends CategorizedAppsState {
	activeView: AppView
	favoriteAppIds: string[]
	firstSeenAt: Record<string, number>
	hiddenAppIds: string[]
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
	const catalogApps = useMemo(() => {
		const hidden = new Set(state.hiddenAppIds)
		return categorizedApps.filter(
			app => !hidden.has(app.id) && !isCatalogArtifact(app),
		)
	}, [categorizedApps, state.hiddenAppIds])
	const paletteApps = useMemo(
		() => catalogApps.filter(app => app.visibilityClass !== 'auxiliary'),
		[catalogApps],
	)
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
		paletteApps,
		visibleHydrationIds,
	}
}
