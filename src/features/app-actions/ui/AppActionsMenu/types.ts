import type { LucideIcon } from 'lucide-react'
import type { ReactNode, RefObject } from 'react'
import type { AppInfo } from '../../../../entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../entities/category'

export interface AppActionsMenuProps {
	app: AppInfo
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onClose(): void
	onMove(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	isHidden?: boolean
	isUserPromoted?: boolean
	onHide(id: string): void
	onRestore(id: string): void
	onDemote(id: string): void
	anchorRef: RefObject<HTMLButtonElement | null>
}

export interface MenuItemProps {
	icon: LucideIcon
	label: ReactNode
	onClick?(): void
	tone?: 'default' | 'danger'
	disabled?: boolean
	iconClassName?: string
	withSpotlight?: boolean
}

export interface CategorySubmenuProps {
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	activeCategory: AppCategory
	onSelect(category: AppCategory): void
}
