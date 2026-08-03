import type { AppCategory, AppInfo, CategoryDefinition } from '../../../types'

export interface InstallersDocsGridProps {
	apps: AppInfo[]
	hasQuery: boolean
	favoriteAppIds: string[]
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	onMoveApp(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onHide(id: string): void
	onRestore(id: string): void
	onDemoteAuxiliary(id: string): void
}

export interface ArtifactSectionProps
	extends Omit<InstallersDocsGridProps, 'apps' | 'hasQuery'> {
	title: string
	apps: AppInfo[]
}
