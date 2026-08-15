import type { AppInfo } from '../../../entities/app'
import type { CategoryDefinition } from '../../../entities/category'
import type { Scenario, ScenarioList } from '../../../entities/scenario'
import type { ScenarioRunProgress } from '../../../features/run-scenario'

export interface ScenariosPageProps {
	scenarios: Scenario[]
	apps: AppInfo[]
	selectableApps: AppInfo[]
	categories: CategoryDefinition[]
	runningId: string | null
	isScenarioRunning: boolean
	runProgress?: ScenarioRunProgress | null
	favoriteScenarioIds: string[]
	onBack(): void
	onCreate(
		name: string,
	): { ok: true; id: string } | { ok: false; error: string }
	onRename(
		id: string,
		name: string,
	): { ok: true } | { ok: false; error: string }
	onDelete(id: string): void
	onAddApp(
		id: string,
		list: ScenarioList,
		identity: string,
	): { ok: true } | { ok: false; error: string }
	onRemoveApp(id: string, list: ScenarioList, identity: string): void
	onRun(scenario: Scenario): void
	onToggleFavorite(id: string): void
}
