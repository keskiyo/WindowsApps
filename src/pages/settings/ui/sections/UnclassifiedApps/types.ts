import type { AppInfo } from '../../../../../entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../../entities/category'

export interface SignalRow {
	label: string
	value: string
}

export interface UnclassifiedAppsProps {
	apps: AppInfo[]
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onMoveApp(appId: string, category: AppCategory): void
}

export interface UnclassifiedRowProps {
	app: AppInfo
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onMoveApp(appId: string, category: AppCategory): void
}
