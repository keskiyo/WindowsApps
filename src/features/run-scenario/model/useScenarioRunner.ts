import { useCallback, useEffect, useRef, useState } from 'react'
import type { AppInfo, CloseAppsResult, CloseProgress } from '../../../entities/app'
import {
	MAX_SCENARIO_ENTRIES,
	resolveScenarioApps,
	type Scenario,
} from '../../../entities/scenario'
import { closeStageLabel } from '../lib/closeStageLabel'
import type { ScenarioRunProgress, ScenarioRunSummary } from '../types'

interface RunnerOptions {
	apps: AppInfo[]
	scenarios: Scenario[]
	launch(app: AppInfo): Promise<void>
	closeApps(ids: string[]): Promise<CloseAppsResult>
	onCloseProgress?(
		handler: (progress: CloseProgress) => void,
	): Promise<() => void>
	onFinished?(summary: ScenarioRunSummary): void
}

export function useScenarioRunner({
	apps,
	scenarios,
	launch,
	closeApps,
	onCloseProgress,
	onFinished,
}: RunnerOptions) {
	const [runningId, setRunningId] = useState<string | null>(null)
	const [progress, setProgress] = useState<ScenarioRunProgress | null>(null)
	const activeRef = useRef(false)

	useEffect(() => {
		if (!onCloseProgress) return
		let active = true
		let stop: (() => void) | undefined
		void onCloseProgress(stage => {
			if (!active || !activeRef.current) return
			setProgress(current =>
				current?.phase === 'closing'
					? { ...current, detail: closeStageLabel(stage) }
					: current,
			)
		})
			.then(unsubscribe => {
				if (active) stop = unsubscribe
				else unsubscribe()
			})
			.catch(() => {})
		return () => {
			active = false
			stop?.()
		}
	}, [onCloseProgress])

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
			const summary: ScenarioRunSummary = {
				scenarioName: scenario.name,
				launched: 0,
				launchFailed: 0,
				closed: 0,
				notRunning: 0,
				blocked: 0,
				closeFailed: 0,
				closeUnavailable: false,
			}
			setProgress({
				phase: 'launching',
				completed: 0,
				total: toLaunch.apps.length,
			})
			try {
				for (const [index, app] of toLaunch.apps.entries()) {
					try {
						await launch(app)
						summary.launched += 1
					} catch {
						summary.launchFailed += 1
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
						const result = await closeApps(
							toClose.apps.map(app => app.id),
						)
						summary.closed = result.closed
						summary.notRunning = result.notRunning
						summary.blocked = result.blocked ?? 0
						summary.closeFailed = result.failed
					} catch {
						summary.closeUnavailable = true
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
				onFinished?.(summary)
			}
		},
		[apps, closeApps, launch, onFinished],
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
