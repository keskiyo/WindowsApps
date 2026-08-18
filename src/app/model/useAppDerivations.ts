import { useMemo } from 'react'
import {
	type AppInfo,
	appIdentity,
	selectRecentApps,
	selectUnclassifiedApps,
} from '../../entities/app'
import { type Scenario, filterFavoriteScenarios } from '../../entities/scenario'

const RECENT_APPS = 20

interface AppDerivationsInput {
	catalogApps: AppInfo[]
	primaryApps: AppInfo[]
	firstSeenAt: Record<string, number>
	scenarios: Scenario[]
	favoriteScenarioIds: string[]
}

export interface RecentApp {
	app: AppInfo
	firstSeenAt: number | null
}

export function useAppDerivations({
	catalogApps,
	primaryApps,
	firstSeenAt,
	scenarios,
	favoriteScenarioIds,
}: AppDerivationsInput) {
	const unclassifiedApps = useMemo(
		() => selectUnclassifiedApps(primaryApps),
		[primaryApps],
	)

	const favoriteScenarios = useMemo(
		() => filterFavoriteScenarios(scenarios, favoriteScenarioIds),
		[favoriteScenarioIds, scenarios],
	)

	const recentApps = useMemo<RecentApp[]>(() => {
		const seenAt = (app: AppInfo) => firstSeenAt[appIdentity(app)] ?? 0
		const dated = catalogApps.filter(app => seenAt(app) > 0)
		return selectRecentApps(dated, seenAt, RECENT_APPS).map(app => ({
			app,
			firstSeenAt: seenAt(app),
		}))
	}, [catalogApps, firstSeenAt])

	return { unclassifiedApps, favoriteScenarios, recentApps }
}
