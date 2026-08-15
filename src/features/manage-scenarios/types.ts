import type { AppInfo, CloseRiskBadge } from '../../entities/app'
import type { CategoryDefinition } from '../../entities/category'
import type {
	Scenario,
	ScenarioList,
	UnavailableScenarioApp,
} from '../../entities/scenario'

export type ScenarioNameResult = { ok: true } | { ok: false; error: string }

export interface CloseRiskMark {
	badge: CloseRiskBadge
	reason: string
}

export interface AppPickerDialogProps {
	apps: AppInfo[]
	categories: CategoryDefinition[]
	list: ScenarioList
	scenarioName: string
	noteOf?(app: AppInfo): string | null
	markOf?(app: AppInfo): CloseRiskMark | null
	onConfirm(apps: AppInfo[]): void
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
	remove?: { label: string; disabled: boolean; onRemove(): void }
	mark?: CloseRiskMark | null
}

export interface UnavailableScenarioAppTileProps {
	entry: UnavailableScenarioApp
	remove?: { label: string; disabled: boolean; onRemove(): void }
}

export interface ScenarioCardProps {
	scenario: Scenario
	apps: AppInfo[]
	selectableApps: AppInfo[]
	categories: CategoryDefinition[]
	running: boolean
	isScenarioRunning: boolean
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
	unavailable: UnavailableScenarioApp[]
	disabled: boolean
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
