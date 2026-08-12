import type { RefObject } from 'react'
import type { ScanProgress } from '../../entities/app'

export interface HeaderProps {
	primaryAppCount: number
	auxiliaryToolCount: number
	visibleCount: number
	query: string
	isRefreshing: boolean
	scanProgress: ScanProgress | null
	menuButtonRef: RefObject<HTMLButtonElement>
	searchInputRef?: RefObject<HTMLInputElement>
	onOpenNavigation(): void
	onQueryChange(query: string): void
	onRefresh(): Promise<void>
	onCancelScan(): Promise<void>
	showMenu: boolean
}

export interface SearchFieldProps {
	query: string
	searchRef: RefObject<HTMLInputElement>
	onQueryChange(query: string): void
}

export interface ScanButtonProps {
	isRefreshing: boolean
	onRefresh(): Promise<void>
	onCancelScan(): Promise<void>
}
