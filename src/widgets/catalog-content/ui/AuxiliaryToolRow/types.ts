import type {
	AppCategory,
	AppInfo,
	CategoryDefinition,
} from '../../../types'

export interface AuxiliaryToolRowProps {
	app: AppInfo
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onLaunch(app: AppInfo): Promise<void>
	onMove(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onRestore(id: string): void
	onDemote(id: string): void
}
