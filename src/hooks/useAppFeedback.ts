import { useCallback } from 'react'
import { toast } from 'sonner'
import type { AppInfo } from '../types'

interface AppFeedbackOptions {
	onLaunch(app: AppInfo): Promise<void>
	onRefresh(): Promise<void>
	onUninstall(id: string): Promise<void>
}

export function useAppFeedback({
	onLaunch,
	onRefresh,
	onUninstall,
}: AppFeedbackOptions) {
	const launch = useCallback(
		async (app: AppInfo) => {
			try {
				await onLaunch(app)
				toast.success(`${app.name} launched`)
			} catch {
				toast.error(`Could not launch ${app.name}`)
			}
		},
		[onLaunch],
	)

	const refresh = useCallback(async () => {
		try {
			await onRefresh()
			toast.success('Application list refreshed')
		} catch (error) {
			const message = String(error)
			if (message.toLowerCase().includes('cancelled')) {
				toast.info('Application scan cancelled')
			} else {
				toast.error('Could not refresh the application list')
			}
		}
	}, [onRefresh])

	const uninstall = useCallback(
		async (id: string) => {
			try {
				await onUninstall(id)
				toast.success('Application uninstalled')
			} catch (error) {
				toast.error('Could not uninstall the application')
				throw error
			}
		},
		[onUninstall],
	)

	return { launch, refresh, uninstall }
}
