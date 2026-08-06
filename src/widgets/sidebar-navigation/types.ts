import type { ComponentProps, RefObject } from 'react'
import type { AppView } from '../../entities/app'
import type {
	AppCategory,
	CategoryDefinition,
	CustomCategoryAccent,
} from '../../entities/category'
import type { AppNavigation } from './ui/AppNavigation/AppNavigation'

export type CreateCategoryResult =
	{ ok: true; id: string } | { ok: false; error: string }

export interface AppDrawerProps {
	open: boolean
	counts: Map<AppCategory, number>
	categoryOrder: AppCategory[]
	categories: CategoryDefinition[]
	activeView: AppView
	appCount: number
	favoriteCount: number
	triggerRef: RefObject<HTMLButtonElement>
	onGoHome(): void
	onSelectView(view: AppView): void
	onSelectCategory(category: AppCategory): void
	onReorderCategory(active: AppCategory, over: AppCategory): void
	onCreateCategory(label: string): CreateCategoryResult
	onClose(): void
	onExited(): void
}

export type AppSidebarProps = ComponentProps<typeof AppNavigation> & {
	onGoHome(): void
}

export interface NavigationIdentityProps {
	onGoHome(): void
}

export interface CategoryDragOverlayProps {
	category: AppCategory
	count: number
	label: string
	accent?: CustomCategoryAccent
}

export interface SortableNavigationCategoryProps extends CategoryDragOverlayProps {
	isDragPreviewActive?: boolean
	onSelect(category: AppCategory): void
}

export interface NavigationCategoryDragOptions {
	navigationRef: RefObject<HTMLElement>
	onReorderCategory(active: AppCategory, over: AppCategory): void
}
