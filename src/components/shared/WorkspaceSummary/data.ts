import { EyeOff, Star, TableProperties, Wrench } from 'lucide-react'
import type { FilterItem } from './types'

export const TONE_CLASSES: Record<FilterItem['tone'], string> = {
	blue: 'bg-sky-100/80 text-sky-700 ring-sky-500/12',
	amber: 'bg-yellow-300/20 text-yellow-300 ring-yellow-300/18',
	slate: 'bg-slate-200/80 text-slate-600 ring-slate-500/12',
	violet: 'bg-violet-100/80 text-violet-700 ring-violet-500/12',
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
