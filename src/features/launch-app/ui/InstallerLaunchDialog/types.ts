import type { AppInfo } from '../../../../entities/app'

export interface InstallerLaunchDialogProps {
	app: AppInfo
	pending: boolean
	onCancel(): void
	onConfirm(): Promise<void> | void
}
