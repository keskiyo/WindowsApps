import { useCallback, useState } from 'react'
import { toast } from 'sonner'
import { useInstallerLaunch } from '../../features/launch-app'
import { useUninstallFlow } from '../../features/uninstall-app'
import { useAppInfoDialog } from '../../features/view-app-details'
import type { AppInfo, UninstallPreview } from '../../entities/app'
import type { SystemClient } from '../../entities/system'

interface DialogOptions {
	systemClient: Pick<SystemClient, 'logClientError'>
	getUninstallPreview(id: string): Promise<UninstallPreview>
	onLaunch(app: AppInfo): Promise<void>
}

export function useCatalogDialogs({
	systemClient,
	getUninstallPreview,
	onLaunch,
}: DialogOptions) {
	const [paletteOpen, setPaletteOpen] = useState(false)
	const appInfo = useAppInfoDialog()
	const uninstall = useUninstallFlow(getUninstallPreview)
	const installerLaunch = useInstallerLaunch(onLaunch)

	const reportFailure = useCallback(
		(kind: string, detail: string) => {
			toast.error('That panel could not be shown. Try again.')
			void systemClient
				.logClientError?.(kind, detail)
				.catch(() => undefined)
		},
		[systemClient],
	)

	return {
		appInfo,
		installerLaunch,
		uninstall,
		palette: {
			open: paletteOpen,
			toggle: useCallback(() => setPaletteOpen(value => !value), []),
			close: useCallback(() => setPaletteOpen(false), []),
		},
		reportFailure,
	}
}
