import { useCallback, useRef, useState } from 'react'
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
	failed: number
	cancelled: boolean
	startedAt: number
	finishedAt: number
}

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
	onFinished(scenario: Scenario, summary: ScenarioRunSummary): void
}

const NOTHING_CLOSED: CloseAppsResult = {
	closed: 0,
	notRunning: 0,
	unavailable: 0,
	blocked: 0,
	failed: 0,
}

export function useScenarioRunner({
	apps,
	scenarios,
	launch,
	closeApps,
	onFinished,
}: RunnerOptions) {
	const [runningId, setRunningId] = useState<string | null>(null)
	const [progress, setProgress] = useState<ScenarioRunProgress | null>(null)
	const activeRef = useRef(false)
	const cancelledRef = useRef(false)

	const cancel = useCallback(() => {
		if (activeRef.current) cancelledRef.current = true
	}, [])

	const run = useCallback(
		async (scenario: Scenario) => {
			if (activeRef.current) return
			activeRef.current = true
			cancelledRef.current = false
			const startedAt = Date.now()
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
			let launched = 0
			let unavailable = toLaunch.missing + toClose.missing
			let closeResult = NOTHING_CLOSED
			try {
				for (const [index, app] of toLaunch.apps.entries()) {
					if (cancelledRef.current) break
					try {
						await launch(app)
						launched += 1
					} catch (error) {
						void toAppClientError(error)
						unavailable += 1
					}
					setProgress({
						phase: 'launching',
						completed: index + 1,
						total: toLaunch.apps.length,
					})
				}
				if (!cancelledRef.current && toClose.apps.length) {
					setProgress({
						phase: 'closing',
						completed: 0,
						total: toClose.apps.length,
					})
					try {
						closeResult = await closeApps(
							toClose.apps.map(app => app.id),
						)
					} catch (error) {
						void toAppClientError(error)
						unavailable += toClose.apps.length
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
			onFinished(scenario, {
				launched,
				closed: closeResult.closed,
				notRunning: closeResult.notRunning,
				unavailable: unavailable + closeResult.unavailable,
				blocked: closeResult.blocked ?? 0,
				failed: closeResult.failed,
				cancelled: cancelledRef.current,
				startedAt,
				finishedAt: Date.now(),
			})
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

	return {
		run,
		runById,
		cancel,
		runningId,
		isRunning: runningId !== null,
		progress,
	}
}
