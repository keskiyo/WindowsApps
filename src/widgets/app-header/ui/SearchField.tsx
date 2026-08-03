import { Search, X } from 'lucide-react'
import { useSpotlight } from '../../../hooks/useSpotlight'
import { SpotlightLayer } from '../SpotlightLayer'
import type { SearchFieldProps } from './types'

export function SearchField({
	query,
	searchRef,
	isRefreshing,
	scanProgress,
	onQueryChange,
}: SearchFieldProps) {
	const searchSpotlight = useSpotlight()

	return (
		<div className='min-w-0 flex-1'>
			{/* Outside <label> so it doesn't pollute the input's computed accessible name */}
			<span id='search-hint' className='sr-only'>
				Searches app name, publisher, description, and install path
			</span>
			<label
				className='group relative flex w-full items-center rounded-xl'
				onPointerMove={searchSpotlight.onPointerMove}
				onPointerEnter={searchSpotlight.onPointerEnter}
				onPointerLeave={searchSpotlight.onPointerLeave}
			>
				<SpotlightLayer size={150} />
				<Search
					className='pointer-events-none absolute left-4 text-slate-500 group-focus-within:text-violet-600'
					size={18}
				/>
				<span className='sr-only'>Search applications</span>
				<input
					ref={searchRef}
					value={query}
					onChange={event => onQueryChange(event.target.value)}
					placeholder='Search apps…'
					aria-describedby='search-hint'
					className='search-input h-11 w-full rounded-xl border border-white/90 bg-slate-100/75 pl-11 pr-11 text-sm text-slate-800 outline-none placeholder:text-slate-500'
				/>
				{query.length > 0 && (
					<button
						type='button'
						aria-label='Clear search'
						onClick={event => {
							event.preventDefault()
							onQueryChange('')
							searchRef.current?.focus()
						}}
						className='absolute right-2 grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-violet-100/75 hover:text-slate-800 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<X size={16} />
					</button>
				)}
			</label>
			{isRefreshing && scanProgress && (
				<p
					className='mt-1.5 truncate px-1 text-xs text-violet-700'
					aria-live='polite'
					aria-atomic='true'
				>
					{scanProgress.stage}
					{scanProgress.location ? ` · ${scanProgress.location}` : ''}
					{scanProgress.totalRoots > 0
						? ` · ${scanProgress.completedRoots}/${scanProgress.totalRoots}`
						: ''}
				</p>
			)}
		</div>
	)
}
