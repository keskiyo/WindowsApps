import { Menu } from 'lucide-react'
import { useRef } from 'react'
import { ScanButton } from './ScanButton'
import { SearchField } from './SearchField'
import type { HeaderProps } from './types'

export function Header({
	appCount,
	visibleCount,
	query,
	isRefreshing,
	scanProgress,
	menuButtonRef,
	searchInputRef,
	onOpenNavigation,
	onGoHome,
	onQueryChange,
	onRefresh,
	onCancelScan,
	showMenu,
}: HeaderProps) {
	const fallbackRef = useRef<HTMLInputElement>(null)
	const searchRef = searchInputRef ?? fallbackRef
	const trimmedQuery = query.trim()
	const countLabel =
		trimmedQuery.length > 0
			? `${visibleCount} ${visibleCount === 1 ? 'match' : 'matches'}`
			: `${appCount} ${appCount === 1 ? 'app' : 'apps'}`
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
				<button
					type='button'
					aria-label='Go to All Apps'
					onClick={onGoHome}
					className='hidden shrink-0 items-center gap-3 rounded-xl text-left focus-visible:outline-2 focus-visible:outline-offset-3 focus-visible:outline-violet-500 md:flex'
				>
					<img
						src='/app-icon.png'
						alt=''
						className='size-10 rounded-xl object-cover ring-1 ring-inset ring-violet-400/25'
					/>
					<span>
						<span className='block text-[1.05rem] font-semibold tracking-tight'>
							Windows Apps
						</span>
						<span className='block text-xs text-slate-500'>
							{countLabel}
						</span>
					</span>
				</button>
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
