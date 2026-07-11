import { X } from 'lucide-react'
import { useEffect, useRef, type RefObject } from 'react'
import { useBodyScrollLock } from '../../hooks/useBodyScrollLock'
import { useFocusTrap } from '../../hooks/useFocusTrap'
import type {
	AppCategory,
	AppInfo,
	AppView,
	CategoryDefinition,
} from '../../types'
import { AppNavigation } from './AppNavigation'

interface Props {
	open: boolean
	apps: AppInfo[]
	categoryOrder: AppCategory[]
	categories: CategoryDefinition[]
	activeView: AppView
	favoriteCount: number
	hiddenCount?: number
	triggerRef: RefObject<HTMLButtonElement>
	onSelectView(view: AppView): void
	onSelectCategory(category: AppCategory): void
	onReorderCategory(active: AppCategory, over: AppCategory): void
	onCreateCategory(
		label: string,
	): { ok: true; id: string } | { ok: false; error: string }
	onClose(): void
	onExited(): void
}

export function AppDrawer(props: Props) {
	const panelRef = useRef<HTMLElement>(null)
	useBodyScrollLock()
	useFocusTrap(panelRef)
	const { onClose, onExited, open, triggerRef } = props
	useEffect(() => {
		panelRef.current?.querySelector<HTMLButtonElement>('button')?.focus()
		function keydown(event: KeyboardEvent) {
			if (event.key === 'Escape') onClose()
		}
		document.addEventListener('keydown', keydown)
		return () => {
			document.removeEventListener('keydown', keydown)
			triggerRef.current?.focus()
		}
	}, [onClose, triggerRef])
	useEffect(() => {
		if (open) return
		const timeout = window.setTimeout(onExited, 240)
		return () => window.clearTimeout(timeout)
	}, [onExited, open])
	const counts = new Map<AppCategory, number>()
	for (const app of props.apps)
		counts.set(app.category, (counts.get(app.category) ?? 0) + 1)
	return (
		<div className={`drawer-shell fixed inset-0 z-400 ${open ? 'is-open' : 'is-closing'}`}>
			<button
				type='button'
				aria-label='Close navigation backdrop'
				onClick={props.onClose}
				className='drawer-backdrop absolute inset-0 cursor-default bg-slate-700/35 backdrop-blur-[2px]'
			/>
			<aside
				ref={panelRef}
				role='dialog'
				aria-modal='true'
				aria-label='App navigation'
				className='drawer-panel absolute inset-y-0 left-0 flex w-[min(22rem,88vw)] flex-col border-r border-slate-300/70 bg-slate-50 shadow-[24px_0_70px_rgba(50,58,78,.24)]'
			>
				<div className='flex items-center justify-between border-b border-slate-300/65 px-5 py-5'>
					<div>
						<p className='text-sm font-semibold text-slate-800'>
							Library
						</p>
						<p className='mt-1 text-xs text-slate-500'>
							Navigate your applications
						</p>
					</div>
					<button
						type='button'
						aria-label='Close navigation'
						onClick={props.onClose}
						className='grid size-10 place-items-center rounded-xl text-slate-500 hover:bg-violet-100/75 hover:text-slate-800 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<X size={19} />
					</button>
				</div>
				<AppNavigation
					categoryOrder={props.categoryOrder}
					categories={props.categories}
					counts={counts}
					activeView={props.activeView}
					favoriteCount={props.favoriteCount}
					hiddenCount={props.hiddenCount ?? 0}
					onSelectView={props.onSelectView}
					onSelectCategory={props.onSelectCategory}
					onReorderCategory={props.onReorderCategory}
					onCreateCategory={props.onCreateCategory}
				/>
			</aside>
		</div>
	)
}
