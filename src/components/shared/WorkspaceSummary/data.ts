import { EyeOff, Star, TableProperties, Wrench } from 'lucide-react'
import type { FilterItem } from './types'

export const TONE_CLASSES: Record<FilterItem['tone'], string> = {
	blue: 'text-[var(--accent-strong)]',
	amber: 'text-[var(--text-primary)]',
	slate: 'text-[var(--text-muted)]',
	violet: 'text-[var(--accent)]',
}

interface FilterCounts {
	allCount: number
	favoriteCount: number
	hiddenCount: number
	auxiliaryCount: number
}

export function buildFilterItems({
	allCount,
	favoriteCount,
	hiddenCount,
	auxiliaryCount,
}: FilterCounts): FilterItem[] {
	return [
		{
			view: 'all',
			label: 'All applications',
			count: allCount,
			tone: 'blue',
			icon: TableProperties,
		},
		{
			view: 'favorites',
			label: 'Favorites',
			count: favoriteCount,
			tone: 'amber',
			icon: Star,
		},
		{
			view: 'hidden',
			label: 'Hidden',
			count: hiddenCount,
			tone: 'slate',
			icon: EyeOff,
		},
		{
			view: 'auxiliary',
			label: 'Auxiliary tools',
			count: auxiliaryCount,
			tone: 'violet',
			icon: Wrench,
		},
	]
}
