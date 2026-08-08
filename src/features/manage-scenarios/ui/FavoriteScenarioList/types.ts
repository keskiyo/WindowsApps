import type { AppInfo } from '../../../../entities/app'
import type { Scenario } from '../../../../entities/scenario'

export interface FavoriteScenarioListProps {
	scenarios: Scenario[]
	apps: AppInfo[]
	runningId: string | null
	onRun(id: string): void
	onToggleFavorite(id: string): void
}

export interface FavoriteScenarioCardProps {
	scenario: Scenario
	apps: AppInfo[]
	expanded: boolean
	running: boolean
	onToggle(id: string): void
	onRun(id: string): void
	onToggleFavorite(id: string): void
}
