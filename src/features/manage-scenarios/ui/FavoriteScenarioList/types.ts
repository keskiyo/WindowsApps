import type { AppInfo } from '../../../../entities/app'
import type { Scenario } from '../../../../entities/scenario'

export interface FavoriteScenarioListProps {
	scenarios: Scenario[]
	apps: AppInfo[]
	runningId: string | null
	isScenarioRunning: boolean
	onRun(id: string): void
	onCancel?(): void
	onToggleFavorite(id: string): void
}

export interface FavoriteScenarioCardProps {
	scenario: Scenario
	apps: AppInfo[]
	expanded: boolean
	running: boolean
	isScenarioRunning: boolean
	onToggle(id: string): void
	onRun(id: string): void
	onCancel?(): void
	onToggleFavorite(id: string): void
}
