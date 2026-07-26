import { TONE_CLASSES } from './data'
import type { FilterTileProps } from './types'

export function FilterTile({ item, active, onSelect }: FilterTileProps) {
	const Icon = item.icon
	const toneClass = TONE_CLASSES[item.tone]

	return (
		<button
			type='button'
			aria-current={active ? 'page' : undefined}
			aria-label={`${item.label} ${item.count}`}
			onClick={() => onSelect(item.view)}
			className={`flex min-h-18 items-center gap-3 rounded-xl border px-4 py-3 text-left shadow-(--shadow-summary) transition-colors hover:ring-1 hover:ring-violet-400/30 focus-visible:outline-2 focus-visible:outline-violet-500 ${
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
