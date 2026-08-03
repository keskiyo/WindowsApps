import type { AppInfo, AppsClient } from '../../../../entities/app'
import type { CategoryDefinition } from '../../../../entities/category'

export interface AppInfoDialogProps {
	app: AppInfo
	categories: CategoryDefinition[]
	appsClient: Pick<AppsClient, 'getAppDetails' | 'openAppFolder'>
	onClose(): void
}
