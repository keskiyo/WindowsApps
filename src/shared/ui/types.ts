import type { LucideIcon } from 'lucide-react'
import type { ReactNode } from 'react'

export interface ConfirmDialogProps {
	label: string
	title: string
	description: string
	confirmLabel: string
	closeLabel: string
	pending?: boolean
	confirmDisabled?: boolean
	children?: ReactNode
	onConfirm(): void
	onClose(): void
}

export interface SectionHeadingProps {
	icon: LucideIcon
	title: string
	titleId: string
	count: number
	noun: string
	description: string
}

export interface ToggleTrackProps {
	checked: boolean
}

export interface FavoriteStarProps {
	label: string
	pressed: boolean
	className?: string
	onToggle(): void
}
