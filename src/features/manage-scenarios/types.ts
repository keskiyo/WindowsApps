import type { AppInfo, CloseRiskBadge } from '../../entities/app'
import type { Scenario, ScenarioList } from '../../entities/scenario'

export type ScenarioNameResult = { ok: true } | { ok: false; error: string }

export interface CloseRiskMark {
	badge: CloseRiskBadge
	reason: string
}

export interface AppPickerDialogProps {
	apps: AppInfo[]
	label: string
	markOf?(app: AppInfo): CloseRiskMark | null
	onSelect(app: AppInfo): void
	onClose(): void
}

export interface ScenarioCloseWarningProps {
	appName: string
	message: string
	onConfirm(): void
	onCancel(): void
}

export interface ScenarioAppTileProps {
	app: AppInfo
	remove?: { label: string; onRemove(): void }
	mark?: CloseRiskMark | null
}

export interface ScenarioCardProps {
	scenario: Scenario
	apps: AppInfo[]
	running: boolean
	runningStatus?: string
	isFavorite: boolean
	onToggleFavorite(id: string): void
	onRename(id: string, name: string): ScenarioNameResult
	onDelete(id: string): void
	onAddApp(
		id: string,
		list: ScenarioList,
		identity: string,
	): ScenarioNameResult
	onRemoveApp(id: string, list: ScenarioList, identity: string): void
	onRun(scenario: Scenario): void
}

export interface ScenarioListRowProps {
	list: ScenarioList
	label: string
	scenarioName: string
	apps: AppInfo[]
	missing: number
	markOf?(app: AppInfo): CloseRiskMark | null
	onAdd(list: ScenarioList): void
	onRemove(list: ScenarioList, identity: string): void
	identityOf(app: AppInfo): string
}

export interface ScenarioNameEditorProps {
	initialValue?: string
	label: string
	onSave(value: string): string | null
	onCancel(): void
}
