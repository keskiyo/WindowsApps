import type { AppInfo } from '../../../entities/app'
import type { Scenario, ScenarioList } from '../../../entities/scenario'

export interface ScenariosPageProps {
	scenarios: Scenario[]
	/** The catalog a scenario's stored identities resolve against. */
	apps: AppInfo[]
	/** The scenario currently starting apps, if any; its Run button stays out of service. */
	runningId: string | null
	onBack(): void
	onCreate(name: string): { ok: true; id: string } | { ok: false; error: string }
	onRename(id: string, name: string): { ok: true } | { ok: false; error: string }
	onDelete(id: string): void
	onAddApp(
		id: string,
		list: ScenarioList,
		identity: string,
	): { ok: true } | { ok: false; error: string }
	onRemoveApp(id: string, list: ScenarioList, identity: string): void
	onRun(scenario: Scenario): void
}
