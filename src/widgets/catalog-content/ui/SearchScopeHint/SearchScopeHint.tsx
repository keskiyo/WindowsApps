import type { SearchScopeHintProps } from '../../types'
import { SEARCH_SCOPE_AREAS } from './data'

export function SearchScopeHint({
	counts,
	onSelectView,
}: SearchScopeHintProps) {
	const areas = SEARCH_SCOPE_AREAS.filter(area => counts[area.key] > 0)
	if (areas.length === 0) return null
	return (
		<p className="mt-6 flex flex-wrap items-center gap-x-2 gap-y-1 px-1 text-sm text-(--text-muted)">
			<span>Also found elsewhere:</span>
			{areas.map(area => (
				<button
					key={area.view}
					type="button"
					onClick={() => onSelectView(area.view)}
					className="rounded-lg px-2 py-0.5 font-medium text-(--text-primary) underline decoration-(--border-neutral) underline-offset-4 transition-colors hover:bg-(--surface-raised) hover:decoration-(--accent) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
				>
					{counts[area.key]}{' '}
					{counts[area.key] === 1 ? 'match' : 'matches'} in{' '}
					{area.label}
				</button>
			))}
		</p>
	)
}
