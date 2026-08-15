import { useCallback } from 'react'
import { toast } from 'sonner'
import { toAppClientError } from '../../shared/api/tauri/errors'
import type { AppInfo } from '../../entities/app'
import {
	scenarioRunSummaryMessage,
	type ScenarioRunSummary,
} from '../../features/run-scenario'

interface AppFeedbackOptions {
	onLaunch(app: AppInfo): Promise<void>
	onRefresh(): Promise<void>
	onUninstall(id: string): Promise<void>
}

export type UninstallOutcome = 'completed' | 'cancelled' | 'failed'

export function useAppFeedback({
	onLaunch,
	onRefresh,
	onUninstall,
}: AppFeedbackOptions) {
	const launch = useCallback(
		async function launch(app: AppInfo) {
			try {
				await onLaunch(app)
				toast.success(`${app.name} launched`)
			} catch {
				toast.error(`Could not launch ${app.name}`, {
					action: {
						label: 'Retry',
						onClick: () => void launch(app),
					},
				})
			}
		},
		[onLaunch],
	)

	const refresh = useCallback(async () => {
		try {
			await onRefresh()
			toast.success('Application list refreshed')
		} catch (error) {
			if (toAppClientError(error).code === 'SCAN_CANCELLED') {
				toast.info('Application scan cancelled')
			} else {
				toast.error('Could not refresh the application list')
			}
		}
	}, [onRefresh])

	const uninstall = useCallback(
		async (app: AppInfo): Promise<UninstallOutcome> => {
			try {
				await onUninstall(app.id)
				toast.success(`${app.name} uninstalled`)
				return 'completed'
			} catch (error) {
				if (toAppClientError(error).code === 'UNINSTALL_CANCELLED') {
					toast.info(`Uninstall of ${app.name} cancelled`)
					return 'cancelled'
				}
				toast.error(`Could not uninstall ${app.name}`)
				return 'failed'
			}
		},
		[onUninstall],
	)

	const reportScenarioRun = useCallback((summary: ScenarioRunSummary) => {
		const message = scenarioRunSummaryMessage(summary)
		if (!message) return
		const failed =
			summary.launchFailed ||
			summary.closeFailed ||
			summary.blocked ||
			summary.closeUnavailable
		if (failed) toast.warning(message)
		else toast.success(message)
	}, [])

	return { launch, refresh, uninstall, reportScenarioRun }
}
