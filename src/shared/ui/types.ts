import type { LucideIcon } from 'lucide-react'

export interface SectionHeadingProps {
	icon: LucideIcon
	title: string
	titleId: string
	count: number
	noun: string
	description: string
}

export interface FavoriteStarProps {
	label: string
	pressed: boolean
	className?: string
	onToggle(): void
}
