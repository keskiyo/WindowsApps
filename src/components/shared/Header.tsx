import { Menu, RefreshCw, Search, X } from 'lucide-react'
import { useRef, type RefObject } from 'react'
interface Props {
	appCount: number
	query: string
	isRefreshing: boolean
	menuButtonRef: RefObject<HTMLButtonElement>
	onOpenNavigation(): void
	onGoHome(): void
	onQueryChange(query: string): void
	onRefresh(): Promise<void>
	showMenu: boolean
}
export function Header({
	appCount,
	query,
	isRefreshing,
	menuButtonRef,
	onOpenNavigation,
	onGoHome,
	onQueryChange,
	onRefresh,
	showMenu,
}: Props) {
	const searchRef = useRef<HTMLInputElement>(null)
	return (
		<header className='sticky top-0 z-300 border-b border-white/[0.07] bg-slate-950/97 shadow-[0_10px_30px_rgba(2,6,23,0.22)]'>
			<div className='mx-auto flex w-full max-w-375 flex-col gap-4 px-5 py-5 sm:px-8 lg:flex-row lg:items-center'>
				<div className='flex min-w-60 items-center gap-3'>
					{showMenu && (
						<button
							ref={menuButtonRef}
							type='button'
							aria-label='Open navigation'
							onClick={onOpenNavigation}
							className='grid size-10 place-items-center rounded-xl border border-white/8 bg-slate-900 text-slate-300 hover:border-blue-400/30 hover:text-blue-200 focus-visible:outline-2 focus-visible:outline-blue-400'
						>
							<Menu size={19} />
						</button>
					)}
					<button
						type='button'
						aria-label='Go to All Apps'
						onClick={onGoHome}
						className='flex items-center gap-3 rounded-xl text-left focus-visible:outline-2 focus-visible:outline-offset-3 focus-visible:outline-blue-400'
					>
						<img
							src='/app-icon.png'
							alt=''
							className='size-10 rounded-xl object-cover ring-1 ring-inset ring-blue-400/25'
						/>
						<span>
							<span className='block text-[1.05rem] font-semibold tracking-tight'>
								Windows Apps
							</span>
							<span className='block text-xs text-slate-400'>
								{appCount} {appCount === 1 ? 'app' : 'apps'}
							</span>
						</span>
					</button>
				</div>
				<div className='flex flex-1 items-center gap-3 lg:justify-end'>
					<label className='group relative flex w-full max-w-2xl items-center'>
						<Search
							className='pointer-events-none absolute left-4 text-slate-500 group-focus-within:text-blue-400'
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
							className='h-11 w-full rounded-xl border border-white/8 bg-slate-900/75 pl-11 pr-11 text-sm outline-none focus:border-blue-400/50 focus:ring-3 focus:ring-blue-500/10'
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
								className='absolute right-2 grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-slate-800 hover:text-slate-200 focus-visible:outline-2 focus-visible:outline-blue-400'
							>
								<X size={16} />
							</button>
						)}
					</label>
					<button
						type='button'
						disabled={isRefreshing}
						onClick={() => void onRefresh()}
						aria-label={
							isRefreshing
								? 'Scanning applications'
								: 'Scan for apps'
						}
						className='grid size-11 shrink-0 place-items-center rounded-xl bg-blue-500 text-white hover:bg-blue-400 focus-visible:outline-2 focus-visible:outline-blue-400 disabled:opacity-60'
					>
						<RefreshCw
							className={isRefreshing ? 'animate-spin' : ''}
							size={18}
						/>
					</button>
				</div>
			</div>
		</header>
	)
}
