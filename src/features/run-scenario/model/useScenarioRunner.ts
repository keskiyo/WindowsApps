import { useCallback, useState } from 'react'
import type { AppInfo, CloseAppsResult } from '../../../entities/app'
import {
	MAX_SCENARIO_ENTRIES,
	resolveScenarioApps,
	type Scenario,
} from '../../../entities/scenario'
import { toAppClientError } from '../../../shared/api/tauri/errors'

export interface ScenarioRunSummary {
	launched: number
	closed: number
	notRunning: number
	unavailable: number
	blocked: number
}

interface RunnerOptions {
	apps: AppInfo[]
	scenarios: Scenario[]
	launch(app: AppInfo): Promise<void>
	closeApps(ids: string[]): Promise<CloseAppsResult>
	onFinished(scenario: Scenario, summary: ScenarioRunSummary): void
}

const NOTHING_CLOSED: CloseAppsResult = {
	closed: 0,
	notRunning: 0,
	unavailable: 0,
	blocked: 0,
}

export function useScenarioRunner({
	apps,
	scenarios,
	launch,
	closeApps,
	onFinished,
}: RunnerOptions) {
	const [runningId, setRunningId] = useState<string | null>(null)

	const run = useCallback(
		async (scenario: Scenario) => {
			if (runningId) return
			setRunningId(scenario.id)
			const toLaunch = resolveScenarioApps(
				scenario.launchIdentities.slice(0, MAX_SCENARIO_ENTRIES),
				apps,
			)
			const toClose = resolveScenarioApps(
				scenario.closeIdentities.slice(0, MAX_SCENARIO_ENTRIES),
				apps,
			)
			let launched = 0
			let unavailable = toLaunch.missing + toClose.missing
			let closeResult = NOTHING_CLOSED
			try {
				for (const app of toLaunch.apps) {
					try {
						await launch(app)
						launched += 1
					} catch (error) {
						void toAppClientError(error)
						unavailable += 1
					}
				}
				if (toClose.apps.length) {
					try {
						closeResult = await closeApps(
							toClose.apps.map(app => app.id),
						)
					} catch (error) {
						void toAppClientError(error)
						unavailable += toClose.apps.length
					}
				}
			} finally {
				setRunningId(null)
			}
			onFinished(scenario, {
				launched,
				closed: closeResult.closed,
				notRunning: closeResult.notRunning,
				unavailable: unavailable + closeResult.unavailable,
				blocked: closeResult.blocked ?? 0,
			})
		},
		[apps, closeApps, launch, onFinished, runningId],
	)

	const runById = useCallback(
		(id: string) => {
			const scenario = scenarios.find(entry => entry.id === id)
			if (scenario) void run(scenario)
		},
		[run, scenarios],
	)

	return { run, runById, runningId }
}
