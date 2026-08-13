import type { AppInfo } from '../../../../entities/app'
import type { Scenario } from '../../../../entities/scenario'
import type { UnavailableScenarioApp } from '../../../../entities/scenario'

export interface ScenarioRunDialogProps {
	scenarios: Scenario[]
	apps: AppInfo[]
	runningId: string | null
	isScenarioRunning: boolean
	onRun(id: string): void
	onCancel?(): void
	onClose(): void
}

export interface ScenarioRunRowProps {
	scenario: Scenario
	apps: AppInfo[]
	expanded: boolean
	running: boolean
	isScenarioRunning: boolean
	onToggle(id: string): void
	onRun(id: string): void
	onCancel?(): void
}

export interface ScenarioRunListProps {
	label: string
	scenarioName: string
	apps: AppInfo[]
	unavailable: UnavailableScenarioApp[]
}
