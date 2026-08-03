import type { AppInfo, AppsClient, CategoryDefinition } from '../../../types'

export interface AppInfoDialogProps {
	app: AppInfo
	categories: CategoryDefinition[]
	appsClient: Pick<AppsClient, 'getAppDetails' | 'openAppFolder'>
	onClose(): void
}
