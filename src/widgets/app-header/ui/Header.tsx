import { Menu } from 'lucide-react'
import { useRef } from 'react'
import { ScanButton } from './ScanButton'
import { SearchField } from './SearchField'
import type { HeaderProps } from '../types'

export function Header({
	visibleCount,
	query,
	isRefreshing,
	scanProgress,
	menuButtonRef,
	searchInputRef,
	onOpenNavigation,
	onQueryChange,
	onRefresh,
	onCancelScan,
	showMenu,
}: HeaderProps) {
	const fallbackRef = useRef<HTMLInputElement>(null)
	const searchRef = searchInputRef ?? fallbackRef
	const searching = query.trim().length > 0
	return (
		<header className='app-header-glass sticky top-0 z-300 border-b border-slate-300/65 shadow-(--shadow-header)'>
			<div className='mx-auto flex w-full max-w-375 items-start gap-3 px-5 pt-4.75 pb-4 sm:px-8 md:items-center'>
				{showMenu && (
					<button
						ref={menuButtonRef}
						type='button'
						aria-label='Open navigation'
						onClick={onOpenNavigation}
						className='grid size-10 shrink-0 place-items-center rounded-xl border border-white/85 bg-white/65 text-slate-600 shadow-sm hover:border-violet-400/35 hover:text-violet-700 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<Menu size={19} />
					</button>
				)}
				{/* The catalog total moved to the All Apps entry; what only the header can say is
				    how much of it the current query leaves. */}
				{searching && (
					<p className='shrink-0 self-center text-sm text-(--text-muted)'>
						{visibleCount} {visibleCount === 1 ? 'match' : 'matches'}
					</p>
				)}
				<div className='flex min-w-0 flex-1 items-start gap-3 md:items-center'>
					<SearchField
						query={query}
						searchRef={searchRef}
						isRefreshing={isRefreshing}
						scanProgress={scanProgress}
						onQueryChange={onQueryChange}
					/>
					<ScanButton
						isRefreshing={isRefreshing}
						onRefresh={onRefresh}
						onCancelScan={onCancelScan}
					/>
				</div>
			</div>
		</header>
	)
}
