import type { LucideIcon } from 'lucide-react'
import type { AppView } from '../../../../entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../entities/category'

export interface AppNavigationProps {
	categoryOrder: AppCategory[]
	categories: CategoryDefinition[]
	counts: Map<AppCategory, number>
	activeView: AppView
	appCount: number
	favoriteCount: number
	onSelectView(view: AppView): void
	onSelectCategory(category: AppCategory): void
	onCreateCategory(
		label: string,
	): { ok: true; id: string } | { ok: false; error: string }
	onReorderCategory(active: AppCategory, over: AppCategory): void
}

export interface NavItemProps {
	icon: LucideIcon
	label: string
	active: boolean
	count?: number
	className?: string
	onClick(): void
}
