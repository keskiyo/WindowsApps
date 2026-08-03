import type { AppInfo, AppView } from '../../../../entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../entities/category'

export interface AppGridProps {
	apps: AppInfo[]
	isLoading: boolean
	hasQuery: boolean
	activeView: AppView
	categoryOrder: AppCategory[]
	categories: CategoryDefinition[]
	collapsedCategories: AppCategory[]
	favoriteAppIds: string[]
	onToggleCategory(category: AppCategory): void
	onToggleFavorite(id: string): void
	onMoveApp(id: string, category: AppCategory): void
	onRenameCategory(
		id: string,
		label: string,
	): { ok: true } | { ok: false; error: string }
	onDeleteCategory(id: string): { ok: true } | { ok: false; error: string }
	onLaunch(app: AppInfo): Promise<void>
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onHide(id: string): void
	onRestore(id: string): void
	onPromoteAuxiliary(id: string): void
	onDemoteAuxiliary(id: string): void
}
