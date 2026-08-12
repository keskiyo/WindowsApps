import { Menu } from 'lucide-react'
import { useRef } from 'react'
import { ScanButton } from './ScanButton'
import { SearchField } from './SearchField'
import type { HeaderProps } from '../types'

function catalogCountText(primaryAppCount: number, auxiliaryToolCount: number) {
	const appLabel = primaryAppCount === 1 ? 'app' : 'apps'
	const toolLabel = auxiliaryToolCount === 1 ? 'tool' : 'tools'
	return `${primaryAppCount} ${appLabel} · ${auxiliaryToolCount} ${toolLabel}`
}

function statusText({
	primaryAppCount,
	auxiliaryToolCount,
	visibleCount,
	query,
	isRefreshing,
	scanProgress,
}: Pick<
	HeaderProps,
	| 'primaryAppCount'
	| 'auxiliaryToolCount'
	| 'visibleCount'
	| 'query'
	| 'isRefreshing'
	| 'scanProgress'
>) {
	if (isRefreshing && scanProgress) {
		return `${scanProgress.stage}${scanProgress.location ? ` · ${scanProgress.location}` : ''}${scanProgress.totalRoots > 0 ? ` · ${scanProgress.completedRoots}/${scanProgress.totalRoots}` : ''}`
	}
	if (query.trim()) {
		return `${visibleCount} ${visibleCount === 1 ? 'match' : 'matches'} in ${catalogCountText(primaryAppCount, auxiliaryToolCount)}`
	}
	return `${catalogCountText(primaryAppCount, auxiliaryToolCount)} found`
}

export function Header({
	primaryAppCount,
	auxiliaryToolCount,
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
	const status = statusText({
		primaryAppCount,
		auxiliaryToolCount,
		visibleCount,
		query,
		isRefreshing,
		scanProgress,
	})
	return (
		<header className="app-header-glass sticky top-0 z-300 border-b border-slate-300/65 shadow-(--shadow-header)">
			<div className="mx-auto flex w-full max-w-375 items-start gap-3 px-5 pt-4.75 pb-4 sm:px-8">
				{showMenu && (
					<button
						ref={menuButtonRef}
						type="button"
						aria-label="Open navigation"
						onClick={onOpenNavigation}
						className="grid size-10 shrink-0 place-items-center rounded-xl border border-white/85 bg-white/65 text-slate-600 shadow-sm hover:border-violet-400/35 hover:text-violet-700 focus-visible:outline-2 focus-visible:outline-violet-500"
					>
						<Menu size={19} />
					</button>
				)}
				<div className="flex min-w-0 flex-1 items-start gap-3">
					<div className="min-w-0 flex-1">
						<SearchField
							query={query}
							searchRef={searchRef}
							onQueryChange={onQueryChange}
						/>
						<p
							role="status"
							aria-live="polite"
							aria-atomic="true"
							className={`mt-1.5 truncate px-1 text-xs ${isRefreshing ? 'text-violet-700' : 'text-(--text-muted)'}`}
						>
							{status}
						</p>
					</div>
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
