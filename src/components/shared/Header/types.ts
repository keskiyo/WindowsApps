import type { RefObject } from 'react'
import type { ScanProgress } from '../../../types'

export interface HeaderProps {
	appCount: number
	visibleCount: number
	query: string
	isRefreshing: boolean
	scanProgress: ScanProgress | null
	menuButtonRef: RefObject<HTMLButtonElement>
	searchInputRef?: RefObject<HTMLInputElement>
	onOpenNavigation(): void
	onGoHome(): void
	onQueryChange(query: string): void
	onRefresh(): Promise<void>
	onCancelScan(): Promise<void>
	showMenu: boolean
}

export interface SearchFieldProps {
	query: string
	searchRef: RefObject<HTMLInputElement>
	isRefreshing: boolean
	scanProgress: ScanProgress | null
	onQueryChange(query: string): void
}

export interface ScanButtonProps {
	isRefreshing: boolean
	onRefresh(): Promise<void>
	onCancelScan(): Promise<void>
}
