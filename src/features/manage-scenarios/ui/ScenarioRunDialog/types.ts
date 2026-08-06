import type { AppInfo } from '../../../../entities/app'
import type { Scenario } from '../../../../entities/scenario'

export interface ScenarioRunDialogProps {
	scenarios: Scenario[]
	apps: AppInfo[]
	runningId: string | null
	onRun(id: string): void
	onClose(): void
}

export interface ScenarioRunRowProps {
	scenario: Scenario
	apps: AppInfo[]
	expanded: boolean
	running: boolean
	onToggle(id: string): void
	onRun(id: string): void
}

export interface ScenarioRunListProps {
	label: string
	scenarioName: string
	apps: AppInfo[]
	missing: number
}
