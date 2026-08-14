import {
	type AppInfo,
	type AppView,
	appIdentity,
	createMarkLookup,
	filterVisibleApps,
	selectRecentApps,
} from '../../../entities/app'
import {
	type Scenario,
	type ScenarioSummary,
	sortScenariosByNewest,
	summarizeScenario,
} from '../../../entities/scenario'

const MORE_PREVIEW = 3
const MORE_SCENARIO_PREVIEW = MORE_PREVIEW - 1

export interface MorePreviewItem {
	app: AppInfo
	firstSeenAt: number | null
}

export interface MorePreview {
	auxiliary: MorePreviewItem[]
	hidden: MorePreviewItem[]
	installersDocs: MorePreviewItem[]
	scenarios: ScenarioSummary[]
}

interface PreviewInput {
	categorizedApps: AppInfo[]
	favoriteAppIds: string[]
	firstSeenAt: Record<string, number>
	hiddenAppIds: string[]
	scenarios: Scenario[]
}

export function buildMorePreview({
	categorizedApps,
	favoriteAppIds,
	firstSeenAt,
	hiddenAppIds,
	scenarios,
}: PreviewInput): MorePreview {
	const isHidden = createMarkLookup(hiddenAppIds, [])
	const isFavorite = createMarkLookup(favoriteAppIds, [])
	const area = (view: AppView) =>
		filterVisibleApps(categorizedApps, view, isHidden, isFavorite)
	const seenAt = (app: AppInfo): number | null => {
		const value = firstSeenAt[appIdentity(app)]
		return Number.isFinite(value) && value > 0 ? value : null
	}
	const firstSeen = (app: AppInfo) => seenAt(app) ?? 0
	const previewItems = (apps: AppInfo[]): MorePreviewItem[] =>
		apps.map(app => ({ app, firstSeenAt: seenAt(app) }))
	const hiddenOrder = new Map(hiddenAppIds.map((id, index) => [id, index]))

	return {
		auxiliary: previewItems(
			selectRecentApps(area('auxiliary'), firstSeen, MORE_PREVIEW),
		),
		hidden: previewItems(
			selectRecentApps(
				area('hidden'),
				app => hiddenOrder.get(app.id) ?? -1,
				MORE_PREVIEW,
			),
		),
		installersDocs: previewItems(
			selectRecentApps(area('installers_docs'), firstSeen, MORE_PREVIEW),
		),
		scenarios: sortScenariosByNewest(
			scenarios.map(summarizeScenario),
		).slice(0, MORE_SCENARIO_PREVIEW),
	}
}
