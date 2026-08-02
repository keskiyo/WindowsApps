import type { AppInfo } from '../../../types'

export interface InstallerLaunchDialogProps {
	app: AppInfo
	pending: boolean
	onCancel(): void
	onConfirm(): Promise<void> | void
}
