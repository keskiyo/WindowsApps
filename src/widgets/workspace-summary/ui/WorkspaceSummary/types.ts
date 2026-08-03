import type { LucideIcon } from 'lucide-react'
import type { AppView } from '../../../../entities/app'

export interface FilterItem {
	view: Exclude<AppView, 'settings'>
	label: string
	count: number
	icon: LucideIcon
	tone: 'blue' | 'amber' | 'slate' | 'violet'
}

export interface WorkspaceSummaryProps {
	activeView: AppView
	allCount: number
	favoriteCount: number
	hiddenCount: number
	auxiliaryCount: number
	onSelectView(view: AppView): void
}

export interface FilterTileProps {
	item: FilterItem
	active: boolean
	onSelect(view: AppView): void
}
