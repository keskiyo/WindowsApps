import { Play, PowerOff, Search } from 'lucide-react'
import type { AppPickerHeaderProps } from './types'

export function AppPickerHeader({
	list,
	scenarioName,
	label,
	query,
	inputRef,
	onQueryChange,
}: AppPickerHeaderProps) {
	const ListIcon = list === 'launch' ? Play : PowerOff
	return (
		<>
			<div className="flex items-center gap-2 border-b border-(--border-neutral) px-4 py-3">
				<span
					className={`inline-flex shrink-0 items-center gap-1.5 rounded-lg border px-2 py-1 text-xs font-semibold text-(--text-primary) ${list === 'launch' ? 'border-(--accent)' : 'border-(--category-orange)'}`}
				>
					<ListIcon size={13} aria-hidden="true" />
					{list === 'launch' ? 'Launch list' : 'Close list'}
				</span>
				<h2 className="min-w-0 flex-1 truncate text-sm text-(--text-muted)">
					{scenarioName}
				</h2>
			</div>
			<div className="flex items-center gap-3 border-b border-(--border-neutral) px-4">
				<Search size={18} aria-hidden="true" />
				<input
					ref={inputRef}
					type="search"
					value={query}
					onChange={event => onQueryChange(event.target.value)}
					placeholder="Search apps…"
					aria-controls="scenario-picker-list"
					aria-label={label}
					className="h-12 w-full bg-transparent text-sm text-(--text-primary) outline-none placeholder:text-(--text-subtle)"
				/>
			</div>
		</>
	)
}
