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
			className={`flex min-h-16 min-w-0 items-center gap-2 rounded-xl border px-3 py-2.5 text-left transition-[background-color,border-color,box-shadow] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--accent-strong)] motion-reduce:transition-none ${
				active
					? 'border-[var(--accent)] bg-[var(--utility-accent)] text-[var(--text-primary)] shadow-[var(--shadow-accent)]'
					: 'border-[var(--border-neutral)] bg-[var(--surface-panel)] text-[var(--text-primary)] shadow-[var(--shadow-summary)] hover:bg-[var(--surface-raised)]'
			}`}
		>
			<span
				className={`grid size-9 shrink-0 place-items-center rounded-lg border border-[var(--border-neutral)] bg-[var(--surface-raised)] ${toneClass}`}
			>
				<Icon size={18} aria-hidden='true' />
			</span>
			<span className='min-w-0'>
				<span className='block truncate text-xs font-medium text-[var(--text-muted)]'>
					{item.label}
				</span>
				<span
					className='block truncate text-base font-semibold text-[var(--text-primary)]'
				>
					{item.count}
				</span>
			</span>
		</button>
	)
}
