import { Menu, RefreshCw, Search, X } from 'lucide-react'
import { useRef, type RefObject } from 'react'
import { useSpotlight } from '../../hooks/useSpotlight'
import type { ScanProgress } from '../../types'
import { SpotlightLayer } from './SpotlightLayer'
interface Props {
	appCount: number
	query: string
	isRefreshing: boolean
	scanProgress: ScanProgress | null
	menuButtonRef: RefObject<HTMLButtonElement>
	onOpenNavigation(): void
	onGoHome(): void
	onQueryChange(query: string): void
	onRefresh(): Promise<void>
	onCancelScan(): Promise<void>
	showMenu: boolean
}
export function Header({
	appCount,
	query,
	isRefreshing,
	scanProgress,
	menuButtonRef,
	onOpenNavigation,
	onGoHome,
	onQueryChange,
	onRefresh,
	onCancelScan,
	showMenu,
}: Props) {
	const searchRef = useRef<HTMLInputElement>(null)
	const searchSpotlight = useSpotlight()
	const scanSpotlight = useSpotlight()
	return (
		<header className='app-header-glass sticky top-0 z-300 border-b border-slate-300/65 shadow-[0_10px_30px_rgba(74,82,105,0.08)]'>
			<div className='mx-auto flex w-full max-w-375 flex-col gap-4 px-5 py-5 sm:px-8 md:flex-row md:items-center'>
				<div className='flex min-w-60 items-center gap-3'>
					{showMenu && (
						<button
							ref={menuButtonRef}
							type='button'
							aria-label='Open navigation'
							onClick={onOpenNavigation}
							className='grid size-10 place-items-center rounded-xl border border-white/85 bg-white/65 text-slate-600 shadow-sm hover:border-violet-400/35 hover:text-violet-700 focus-visible:outline-2 focus-visible:outline-violet-500'
						>
							<Menu size={19} />
						</button>
					)}
					<button
						type='button'
						aria-label='Go to All Apps'
						onClick={onGoHome}
						className='flex items-center gap-3 rounded-xl text-left focus-visible:outline-2 focus-visible:outline-offset-3 focus-visible:outline-violet-500'
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
								{appCount} {appCount === 1 ? 'app' : 'apps'}
							</span>
						</span>
					</button>
				</div>
				<div className='flex flex-1 items-start gap-3 md:items-center md:justify-end'>
					<div className='w-full max-w-2xl'>
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
								onChange={event =>
									onQueryChange(event.target.value)
								}
								placeholder='Search apps…'
								className='h-11 w-full rounded-xl border border-white/90 bg-slate-100/75 pl-11 pr-11 text-sm text-slate-800 shadow-[inset_2px_2px_5px_rgba(100,112,138,.12),inset_-2px_-2px_5px_rgba(255,255,255,.9)] outline-none placeholder:text-slate-400 focus:border-violet-400/55 focus:ring-3 focus:ring-violet-500/10'
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
							>
								{scanProgress.stage}
								{scanProgress.location
									? ` · ${scanProgress.location}`
									: ''}
								{scanProgress.totalRoots > 0
									? ` · ${scanProgress.completedRoots}/${scanProgress.totalRoots}`
									: ''}
							</p>
						)}
					</div>
					<button
						type='button'
						onClick={() =>
							void (isRefreshing ? onCancelScan() : onRefresh())
						}
						aria-label={
							isRefreshing ? 'Cancel scan' : 'Scan for apps'
						}
						onPointerMove={scanSpotlight.onPointerMove}
						onPointerEnter={scanSpotlight.onPointerEnter}
						onPointerLeave={scanSpotlight.onPointerLeave}
						className={`relative grid size-11 shrink-0 place-items-center rounded-xl text-white shadow-[0_8px_18px_rgba(104,69,216,.22)] focus-visible:outline-2 ${isRefreshing ? 'bg-red-500 hover:bg-red-400 focus-visible:outline-red-300' : 'bg-violet-600 hover:bg-violet-500 focus-visible:outline-violet-500'}`}
					>
						<SpotlightLayer size={70} />
						{isRefreshing ? (
							<X size={18} />
						) : (
							<RefreshCw size={18} />
						)}
					</button>
				</div>
			</div>
		</header>
	)
}
