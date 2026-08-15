import type { Scenario } from '../../../../entities/scenario'
import type { ScenarioNameResult } from '../../types'

export interface ScenarioCardHeaderProps {
	scenario: Scenario
	running: boolean
	isScenarioRunning: boolean
	runningStatus?: string
	isFavorite: boolean
	onToggleFavorite(id: string): void
	onRename(id: string, name: string): ScenarioNameResult
	onDelete(id: string): void
	onRun(scenario: Scenario): void
}
