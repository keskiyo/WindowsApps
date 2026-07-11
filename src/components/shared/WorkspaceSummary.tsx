import { EyeOff, FolderKanban, Search, Star, TableProperties } from 'lucide-react'

interface SummaryItem {
	label: string
	value: string
	tone: 'blue' | 'amber' | 'slate' | 'violet'
	icon: typeof TableProperties
}

interface WorkspaceSummaryProps {
	visibleCount: number
	activeCategoryCount: number
	favoriteCount: number
	hiddenCount: number
	hasQuery: boolean
}

export function WorkspaceSummary({
	visibleCount,
	activeCategoryCount,
	favoriteCount,
	hiddenCount,
	hasQuery,
}: WorkspaceSummaryProps) {
	const items: SummaryItem[] = [
		{
			label: hasQuery ? 'Search results' : 'Visible apps',
			value: String(visibleCount),
			tone: 'blue',
			icon: hasQuery ? Search : TableProperties,
		},
		{
			label: 'Categories',
			value: `${activeCategoryCount} active`,
			tone: 'violet',
			icon: FolderKanban,
		},
		{
			label: 'Favorites',
			value: String(favoriteCount),
			tone: 'amber',
			icon: Star,
		},
		{
			label: 'Hidden',
			value: String(hiddenCount),
			tone: 'slate',
			icon: EyeOff,
		},
	]
	return (
		<section
			aria-label='Workspace summary'
			className='mb-6 grid gap-2.5 sm:grid-cols-2 xl:grid-cols-4'
		>
			{items.map(item => (
				<SummaryTile key={item.label} item={item} />
			))}
		</section>
	)
}

function SummaryTile({ item }: { item: SummaryItem }) {
	const Icon = item.icon
	const toneClass = {
		blue: 'bg-sky-100/80 text-sky-700 ring-sky-500/12',
		amber: 'bg-yellow-300/20 text-yellow-300 ring-yellow-300/18',
		slate: 'bg-slate-200/80 text-slate-600 ring-slate-500/12',
		violet: 'bg-violet-100/80 text-violet-700 ring-violet-500/12',
	}[item.tone]
	return (
		<div className='flex min-h-18 items-center gap-3 rounded-xl border border-white/80 bg-white/62 px-4 py-3 shadow-[0_10px_22px_rgba(74,82,105,0.07)]'>
			<span
				className={`grid size-10 shrink-0 place-items-center rounded-xl ring-1 ring-inset ${toneClass}`}
			>
				<Icon size={18} aria-hidden='true' />
			</span>
			<span className='min-w-0'>
				<span className='block truncate text-xs font-medium text-slate-500'>
					{item.label}
				</span>
				<span className='block truncate text-base font-semibold text-slate-800'>
					{item.value}
				</span>
			</span>
		</div>
	)
}
