import { EyeOff, Star, TableProperties, Wrench } from 'lucide-react'
import type { AppView } from '../../types'

interface FilterItem {
	view: Exclude<AppView, 'settings'>
	label: string
	count: number
	icon: typeof TableProperties
	tone: 'blue' | 'amber' | 'slate' | 'violet'
}

interface WorkspaceSummaryProps {
	activeView: AppView
	allCount: number
	favoriteCount: number
	hiddenCount: number
	auxiliaryCount: number
	onSelectView(view: AppView): void
}

export function WorkspaceSummary({
	activeView,
	allCount,
	favoriteCount,
	hiddenCount,
	auxiliaryCount,
	onSelectView,
}: WorkspaceSummaryProps) {
	const items: FilterItem[] = [
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

	return (
		<section
			aria-label='Catalog filters'
			className='mb-6 grid grid-cols-2 gap-2.5 xl:grid-cols-4'
		>
			{items.map(item => (
				<FilterTile
					key={item.view}
					item={item}
					active={activeView === item.view}
					onSelect={onSelectView}
				/>
			))}
		</section>
	)
}

function FilterTile({
	item,
	active,
	onSelect,
}: {
	item: FilterItem
	active: boolean
	onSelect(view: AppView): void
}) {
	const Icon = item.icon
	const toneClass = {
		blue: 'bg-sky-100/80 text-sky-700 ring-sky-500/12',
		amber: 'bg-yellow-300/20 text-yellow-300 ring-yellow-300/18',
		slate: 'bg-slate-200/80 text-slate-600 ring-slate-500/12',
		violet: 'bg-violet-100/80 text-violet-700 ring-violet-500/12',
	}[item.tone]

	return (
		<button
			type='button'
			aria-current={active ? 'page' : undefined}
			aria-label={`${item.label} ${item.count}`}
			onClick={() => onSelect(item.view)}
			className={`flex min-h-18 items-center gap-3 rounded-xl border px-4 py-3 text-left shadow-[var(--shadow-summary)] transition-colors hover:ring-1 hover:ring-violet-400/30 focus-visible:outline-2 focus-visible:outline-violet-500 ${
				active
					? 'border-violet-400/55 bg-violet-100/80 text-violet-800'
					: 'border-white/80 bg-white/62 text-slate-800 hover:border-violet-300/45 hover:bg-violet-100/45'
			}`}
		>
			<span
				className={`grid size-10 shrink-0 place-items-center rounded-xl ring-1 ring-inset ${toneClass}`}
			>
				<Icon size={18} aria-hidden='true' />
			</span>
			<span className='min-w-0'>
				<span className='block truncate text-xs font-medium text-slate-500'>
					{item.label}
				</span>
				<span
					className={`block truncate text-base font-semibold ${active ? 'text-white' : ''}`}
				>
					{item.count}
				</span>
			</span>
		</button>
	)
}
