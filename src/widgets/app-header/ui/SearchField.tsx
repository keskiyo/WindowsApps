import { Search, X } from 'lucide-react'
import { useSpotlight } from '../../../shared/hooks/useSpotlight'
import { SpotlightLayer } from '../../../shared/ui/SpotlightLayer'
import type { SearchFieldProps } from '../types'

export function SearchField({
	query,
	searchRef,
	onQueryChange,
}: SearchFieldProps) {
	const searchSpotlight = useSpotlight()

	return (
		<div className="min-w-0 flex-1">
			<span id="search-hint" className="sr-only">
				Searches app name, publisher, version, description, and install path
			</span>
			<label
				className="group relative flex w-full items-center rounded-xl"
				onPointerMove={searchSpotlight.onPointerMove}
				onPointerEnter={searchSpotlight.onPointerEnter}
				onPointerLeave={searchSpotlight.onPointerLeave}
			>
				<SpotlightLayer size={150} />
				<Search
					className="pointer-events-none absolute left-4 text-slate-500 group-focus-within:text-violet-600"
					size={18}
				/>
				<span className="sr-only">Search applications</span>
				<input
					ref={searchRef}
					value={query}
					onChange={event => onQueryChange(event.target.value)}
					onKeyDown={event => {
						if (event.key !== 'Escape' || !query) return
						event.preventDefault()
						onQueryChange('')
					}}
					placeholder="Search apps…"
					aria-describedby="search-hint"
					className={`search-input h-11 w-full rounded-xl border border-white/90 bg-slate-100/75 pl-11 text-sm text-slate-800 outline-none placeholder:text-slate-500 ${query.length > 0 ? 'pr-11' : 'pr-11 sm:pr-[4.75rem]'}`}
				/>
				{query.length === 0 && (
					<kbd
						aria-hidden="true"
						className="pointer-events-none absolute right-3 hidden rounded-md border border-(--border-neutral) bg-(--surface-raised) px-1.5 py-0.5 font-sans text-[0.7rem] font-medium text-(--text-muted) sm:block"
					>
						Ctrl+K
					</kbd>
				)}
				{query.length > 0 && (
					<button
						type="button"
						aria-label="Clear search"
						onClick={event => {
							event.preventDefault()
							onQueryChange('')
							searchRef.current?.focus()
						}}
						className="absolute right-2 grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-violet-100/75 hover:text-slate-800 focus-visible:outline-2 focus-visible:outline-violet-500"
					>
						<X size={16} />
					</button>
				)}
			</label>
		</div>
	)
}
