import type { LucideIcon } from 'lucide-react'
import type {
	CSSProperties,
	KeyboardEvent as ReactKeyboardEvent,
	ReactNode,
	RefObject,
} from 'react'
import type { AppInfo, CatalogArtifactKind } from '../../../../entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../entities/category'

export interface AppActionsMenuProps {
	app: AppInfo
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onClose(): void
	onMove(
		id: string,
		category: AppCategory,
		artifact?: CatalogArtifactKind,
	): void
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
	icon?: LucideIcon
	trailingIcon?: LucideIcon
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
	onExpand(category: AppCategory): void
	expandedCategory: AppCategory | null
	menuRef: RefObject<HTMLDivElement>
	position: Pick<CSSProperties, 'left' | 'top'>
	onKeyDown(event: ReactKeyboardEvent<HTMLDivElement>): void
	label: string
}

export interface ArtifactSubmenuProps {
	onSelect(kind: CatalogArtifactKind): void
	menuRef: RefObject<HTMLDivElement>
	position: Pick<CSSProperties, 'left' | 'top'>
	onKeyDown(event: ReactKeyboardEvent<HTMLDivElement>): void
	label: string
}
