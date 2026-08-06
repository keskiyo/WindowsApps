import type { AppInfo } from '../../../../entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../entities/category'

export interface CategorySectionProps {
	category: AppCategory
	definition: CategoryDefinition
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	apps: AppInfo[]
	collapsed: boolean
	favoriteAppIds: string[]
	onToggle(): void
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	onMoveApp(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onHide(id: string): void
	onRestore(id: string): void
	onDemote(id: string): void
	onRenameCategory(
		id: string,
		label: string,
	): { ok: true } | { ok: false; error: string }
	onDeleteCategory(id: string): { ok: true } | { ok: false; error: string }
}
