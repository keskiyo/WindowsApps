import type { AppCategory, AppInfo, CategoryDefinition } from '../../../types'

export interface AppCardProps {
	app: AppInfo
	isFavorite: boolean
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	onMove(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	isHidden?: boolean
	isAuxiliary?: boolean
	onHide(id: string): void
	onRestore(id: string): void
	onDemote(id: string): void
}

export interface CardIconProps {
	iconBase64: string | null
	launching: boolean
}

export interface CardLabelProps {
	name: string
	version: string | null
	launching: boolean
}

export interface FavoriteButtonProps {
	appName: string
	isFavorite: boolean
	onToggle(): void
}
