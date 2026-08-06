import type { LucideIcon } from 'lucide-react'
import type { AppInfo, AppView } from '../../../../entities/app'
import type { Scenario, ScenarioSummary } from '../../../../entities/scenario'

export interface MorePreviewItem {
	app: AppInfo
	firstSeenAt: number | null
}

/**
 * Everything the page needs to run a scenario without leaving it: the list itself for the full
 * dialog, the catalog its stored identities resolve against, and the runner.
 */
export interface ScenarioRunControl {
	scenarios: Scenario[]
	apps: AppInfo[]
	runningId: string | null
	onRun(id: string): void
}

/** What a scenario preview row can do. `onViewAll` opens the whole list over the page. */
export interface ScenarioPreviewControl {
	runningId: string | null
	onRun(id: string): void
	onViewAll(): void
}

/**
 * What a card previews. A discriminated variant rather than two optional lists: only one of them
 * is ever set, the card picks its row component from `kind` instead of guessing, and the run
 * control travels with the variant that has something to run.
 */
export type MorePreview =
	| { kind: 'apps'; items: MorePreviewItem[] }
	| ({ kind: 'scenarios'; items: ScenarioSummary[] } & ScenarioPreviewControl)

export interface MoreDestination {
	view: AppView
	label: string
	description: string
	count: number
	icon: LucideIcon
	/** The newest entries of the area, previewed above the link to the full view. */
	recent: MorePreview
}

export interface MoreAreaPreview {
	auxiliary: MorePreviewItem[]
	hidden: MorePreviewItem[]
	installersDocs: MorePreviewItem[]
	scenarios: ScenarioSummary[]
}

export interface MorePageProps {
	auxiliaryCount: number
	hiddenCount: number
	installersDocsCount: number
	scenarioCount: number
	preview: MoreAreaPreview
	scenarioRun: ScenarioRunControl
	onSelectView(view: AppView): void
}

export interface MoreCardProps {
	destination: MoreDestination
	onSelect(view: AppView): void
}
