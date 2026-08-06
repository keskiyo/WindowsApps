import {
	type AppInfo,
	type AppView,
	appIdentity,
	filterVisibleApps,
	selectRecentApps,
} from '../../../entities/app'
import {
	type Scenario,
	type ScenarioSummary,
	sortScenariosByNewest,
	summarizeScenario,
} from '../../../entities/scenario'

/** Rows each More card previews before handing over to the full view. */
const MORE_PREVIEW = 3
/**
 * The scenarios card previews one row fewer: it also carries a "View all" row, and the two-column
 * grid pairs it with cards that have none, so matching their row count would leave it taller than
 * its neighbour.
 */
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
	/** The catalog after deduplication and category overrides, as the grids see it. */
	categorizedApps: AppInfo[]
	favoriteAppIds: string[]
	firstSeenAt: Record<string, number>
	hiddenAppIds: string[]
	scenarios: Scenario[]
}

/**
 * The newest entries of each area the More page collects.
 *
 * "Newest" is whatever the area can honestly say, which is not the same thing everywhere: the
 * catalog carries no timestamps, so a hidden app is ordered by when the user hid it, a
 * scanner-owned one by the first-seen stamp, and a scenario by when it was created.
 */
export function buildMorePreview({
	categorizedApps,
	favoriteAppIds,
	firstSeenAt,
	hiddenAppIds,
	scenarios,
}: PreviewInput): MorePreview {
	const area = (view: AppView) =>
		filterVisibleApps(categorizedApps, view, hiddenAppIds, favoriteAppIds)
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
		// Scenarios are the user's own list, so "recent" is when they made them.
		scenarios: sortScenariosByNewest(scenarios.map(summarizeScenario)).slice(
			0,
			MORE_SCENARIO_PREVIEW,
		),
	}
}
