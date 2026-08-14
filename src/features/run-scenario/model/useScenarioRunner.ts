import { useCallback, useRef, useState } from 'react'
import type { AppInfo, CloseAppsResult } from '../../../entities/app'
import {
	MAX_SCENARIO_ENTRIES,
	resolveScenarioApps,
	type Scenario,
} from '../../../entities/scenario'
import { toAppClientError } from '../../../shared/api/tauri/errors'

export interface ScenarioRunProgress {
	phase: 'launching' | 'closing'
	completed: number
	total: number
}

interface RunnerOptions {
	apps: AppInfo[]
	scenarios: Scenario[]
	launch(app: AppInfo): Promise<void>
	closeApps(ids: string[]): Promise<CloseAppsResult>
}

export function useScenarioRunner({
	apps,
	scenarios,
	launch,
	closeApps,
}: RunnerOptions) {
	const [runningId, setRunningId] = useState<string | null>(null)
	const [progress, setProgress] = useState<ScenarioRunProgress | null>(null)
	const activeRef = useRef(false)

	const run = useCallback(
		async (scenario: Scenario) => {
			if (activeRef.current) return
			activeRef.current = true
			setRunningId(scenario.id)
			const toLaunch = resolveScenarioApps(
				scenario.launchIdentities.slice(0, MAX_SCENARIO_ENTRIES),
				apps,
			)
			const toClose = resolveScenarioApps(
				scenario.closeIdentities.slice(0, MAX_SCENARIO_ENTRIES),
				apps,
			)
			setProgress({
				phase: 'launching',
				completed: 0,
				total: toLaunch.apps.length,
			})
			try {
				for (const [index, app] of toLaunch.apps.entries()) {
					try {
						await launch(app)
					} catch (error) {
						void toAppClientError(error)
					}
					setProgress({
						phase: 'launching',
						completed: index + 1,
						total: toLaunch.apps.length,
					})
				}
				if (toClose.apps.length) {
					setProgress({
						phase: 'closing',
						completed: 0,
						total: toClose.apps.length,
					})
					try {
						await closeApps(toClose.apps.map(app => app.id))
					} catch (error) {
						void toAppClientError(error)
					}
					setProgress({
						phase: 'closing',
						completed: toClose.apps.length,
						total: toClose.apps.length,
					})
				}
			} finally {
				activeRef.current = false
				setRunningId(null)
				setProgress(null)
			}
		},
		[apps, closeApps, launch],
	)

	const runById = useCallback(
		(id: string) => {
			const scenario = scenarios.find(entry => entry.id === id)
			if (scenario) void run(scenario)
		},
		[run, scenarios],
	)

	return { run, runById, runningId, isRunning: runningId !== null, progress }
}
