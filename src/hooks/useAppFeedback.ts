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
		async function launch(app: AppInfo) {
			try {
				await onLaunch(app)
				toast.success(`${app.name} launched`)
			} catch (error) {
				const reason = error instanceof Error ? error.message : null
				toast.error(
					reason
						? `Could not launch ${app.name}: ${reason}`
						: `Could not launch ${app.name}`,
					{
						action: {
							label: 'Retry',
							onClick: () => void launch(app),
						},
					},
				)
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
		async (app: AppInfo) => {
			try {
				await onUninstall(app.id)
				toast.success(`${app.name} uninstalled`)
			} catch {
				toast.error(`Could not uninstall ${app.name}`)
				throw app.name // rethrown so App.tsx knows to keep the dialog open
			}
		},
		[onUninstall],
	)

	return { launch, refresh, uninstall }
}
