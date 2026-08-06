/**
 * A named pair of app lists: everything in `launchIdentities` is started and everything in
 * `closeIdentities` is closed when the scenario runs.
 *
 * The lists hold **preference identities**, not catalog ids — the same durable key favorites,
 * hidden apps and first-seen stamps use. A catalog id is a function of the deduplication grouping
 * and changes between releases, so a scenario keyed by id would quietly empty itself after a
 * Force full scan.
 */
export interface Scenario {
	id: string
	name: string
	launchIdentities: string[]
	closeIdentities: string[]
	/**
	 * When the user created it. `null` for a scenario stored before the field existed — the date
	 * is then simply not shown, rather than invented from the migration's own clock.
	 */
	createdAt: number | null
}

/** Which of a scenario's two lists an app is being added to or removed from. */
export type ScenarioList = 'launch' | 'close'

/**
 * Upper bound on one list. Running a scenario spawns one IPC call per entry, and the close path
 * holds a grace period per app, so the work a single click can start has to be bounded.
 */
export const MAX_SCENARIO_ENTRIES = 20

/** Upper bound on the number of scenarios, for the same reason the entries are bounded. */
export const MAX_SCENARIOS = 50

/** What the More card shows about a scenario without resolving its apps against the catalog. */
export interface ScenarioSummary {
	id: string
	name: string
	launchCount: number
	closeCount: number
	createdAt: number | null
}

export function summarizeScenario(scenario: Scenario): ScenarioSummary {
	return {
		id: scenario.id,
		name: scenario.name,
		launchCount: scenario.launchIdentities.length,
		closeCount: scenario.closeIdentities.length,
		createdAt: scenario.createdAt,
	}
}
