import type { LucideIcon } from 'lucide-react'
import type { AppInfo, AppView } from '../../../../entities/app'
import type { Scenario, ScenarioSummary } from '../../../../entities/scenario'

export interface MorePreviewItem {
	app: AppInfo
	firstSeenAt: number | null
}

export interface ScenarioRunControl {
	scenarios: Scenario[]
	apps: AppInfo[]
	runningId: string | null
	isScenarioRunning: boolean
	onRun(id: string): void
	onCancel?(): void
}

export interface ScenarioPreviewControl {
	runningId: string | null
	isScenarioRunning: boolean
	onRun(id: string): void
	onCancel?(): void
	onViewAll(): void
}

export type MorePreview =
	| { kind: 'apps'; items: MorePreviewItem[] }
	| ({ kind: 'scenarios'; items: ScenarioSummary[] } & ScenarioPreviewControl)

export interface MoreDestination {
	view: AppView
	label: string
	description: string
	count: number
	icon: LucideIcon
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
	recentApps?: MorePreviewItem[]
	preview: MoreAreaPreview
	scenarioRun: ScenarioRunControl
	onSelectView(view: AppView): void
}

export interface MoreCardProps {
	destination: MoreDestination
	onSelect(view: AppView): void
}

export interface MorePreviewRowProps {
	entry: MorePreviewItem
}

export interface MoreScenarioRowProps {
	scenario: ScenarioSummary
	running: boolean
	isScenarioRunning: boolean
	onRun(id: string): void
	onCancel?(): void
}
