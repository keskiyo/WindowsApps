import { X } from 'lucide-react'
import { useEffect, useRef, type RefObject } from 'react'
import type {
	AppCategory,
	AppInfo,
	AppView,
	CategoryDefinition,
} from '../types'
import { AppNavigation } from './AppNavigation'

interface Props {
	apps: AppInfo[]
	categoryOrder: AppCategory[]
	categories: CategoryDefinition[]
	activeView: AppView
	favoriteCount: number
	hiddenCount?: number
	triggerRef: RefObject<HTMLButtonElement>
	onSelectView(view: AppView): void
	onSelectCategory(category: AppCategory): void
	onCreateCategory(
		label: string,
	): { ok: true; id: string } | { ok: false; error: string }
	onClose(): void
}

export function AppDrawer(props: Props) {
	const panelRef = useRef<HTMLElement>(null)
	useEffect(() => {
		const previousOverflow = document.body.style.overflow
		document.body.style.overflow = 'hidden'
		panelRef.current?.querySelector<HTMLButtonElement>('button')?.focus()
		function keydown(event: KeyboardEvent) {
			if (event.key === 'Escape') props.onClose()
			if (event.key === 'Tab' && panelRef.current) {
				const items = [
					...panelRef.current.querySelectorAll<HTMLElement>(
						'button:not([disabled])',
					),
				]
				if (!items.length) return
				const first = items[0],
					last = items[items.length - 1]
				if (event.shiftKey && document.activeElement === first) {
					event.preventDefault()
					last.focus()
				} else if (!event.shiftKey && document.activeElement === last) {
					event.preventDefault()
					first.focus()
				}
			}
		}
		document.addEventListener('keydown', keydown)
		return () => {
			document.body.style.overflow = previousOverflow
			document.removeEventListener('keydown', keydown)
			props.triggerRef.current?.focus()
		}
	}, [props.onClose, props.triggerRef])
	const counts = new Map<AppCategory, number>()
	for (const app of props.apps)
		counts.set(app.category, (counts.get(app.category) ?? 0) + 1)
	return (
		<div className='fixed inset-0 z-400'>
			<button
				type='button'
				aria-label='Close navigation backdrop'
				onClick={props.onClose}
				className='absolute inset-0 cursor-default bg-slate-950/70'
			/>
			<aside
				ref={panelRef}
				role='dialog'
				aria-modal='true'
				aria-label='App navigation'
				className='drawer-enter absolute inset-y-0 left-0 flex w-[min(22rem,88vw)] flex-col border-r border-white/8 bg-slate-950 shadow-[24px_0_70px_rgba(0,0,0,.48)]'
			>
				<div className='flex items-center justify-between border-b border-white/8 px-5 py-5'>
					<div>
						<p className='text-sm font-semibold text-slate-100'>
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
						className='grid size-10 place-items-center rounded-xl text-slate-400 hover:bg-slate-800 hover:text-slate-100'
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
					onCreateCategory={props.onCreateCategory}
				/>
			</aside>
		</div>
	)
}
