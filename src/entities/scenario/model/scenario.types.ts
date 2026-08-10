export interface Scenario {
	id: string
	name: string
	launchIdentities: string[]
	closeIdentities: string[]
	createdAt: number | null
}

export interface ScenarioRunRecord {
	id: string
	scenarioId: string
	scenarioName: string
	startedAt: number
	finishedAt: number
	launched: number
	closed: number
	notRunning: number
	unavailable: number
	blocked: number
	failed: number
	cancelled: boolean
}

export type ScenarioList = 'launch' | 'close'

export const MAX_SCENARIO_ENTRIES = 20

export const MAX_SCENARIOS = 50

export const MAX_SCENARIO_HISTORY = 50

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
