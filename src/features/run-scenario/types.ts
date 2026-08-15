export interface ScenarioRunProgress {
	phase: 'launching' | 'closing'
	completed: number
	total: number
	detail?: string
}

export interface ScenarioRunSummary {
	scenarioName: string
	launched: number
	launchFailed: number
	closed: number
	notRunning: number
	blocked: number
	closeFailed: number
	closeUnavailable: boolean
}
