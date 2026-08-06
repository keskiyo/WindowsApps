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
	/** Apps that were already not running — the scenario's goal, not a failure. */
	notRunning: number
	/** Entries that resolved to no catalog app, plus the ones that could not be started or closed. */
	unavailable: number
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
}

/**
 * Runs one scenario: start the launch list, then close the close list.
 *
 * Launch-first matches how the user described it — the apps they asked for appear immediately —
 * and closing afterwards frees the machine while they are starting. Launches are awaited one at a
 * time rather than fired together: each resolves once its process exists, which is what keeps a
 * twenty-app scenario from starting twenty programs at the same instant. The whole close list is
 * one request, because the backend enumerates once and waits out a single grace period for the
 * batch; asking per app would make the user wait that period once per program.
 *
 * Nothing here throws. A scenario is a batch, and one app that refuses to start is reported in the
 * summary instead of aborting the rest.
 */
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
						// Reported through the summary; `toAppClientError` keeps a raw transport
						// value from reaching the interface.
						void toAppClientError(error)
						unavailable += 1
					}
				}
				if (toClose.apps.length) {
					try {
						closeResult = await closeApps(toClose.apps.map(app => app.id))
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
