import type { LucideIcon } from 'lucide-react'
import type { AppInfo } from '../../entities/app'
import type { AppCategory, CategoryDefinition } from '../../entities/category'
import type { Scenario } from '../../entities/scenario'

export interface CatalogViewHeaderProps {
	icon: LucideIcon
	title: string
	titleId: string
	count: number
	noun?: string
	back?: { label: string; onBack(): void }
}

export interface AuxiliaryGridProps {
	apps: AppInfo[]
	hasQuery: boolean
	favoriteAppIds: string[]
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onBack(): void
	onLaunch(app: AppInfo): Promise<void>
	onMoveApp(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onPromote(id: string): void
	onDemote(id: string): void
}

export interface CatalogGridProps {
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
	onDemote(id: string): void
}

export interface FavoriteScenariosPanel {
	scenarios: Scenario[]
	apps: AppInfo[]
	runningId: string | null
	onRun(id: string): void
	onToggleFavorite(id: string): void
}

export interface FavoritesGridProps extends CatalogGridProps {
	favoriteScenarios: FavoriteScenariosPanel
}

export interface HiddenGridProps extends CatalogGridProps {
	onBack(): void
}

export interface CategoryHeaderProps {
	category: AppCategory
	label: string
	appCount: number
	collapsed: boolean
	onToggle(): void
	onEdit(): void
}
