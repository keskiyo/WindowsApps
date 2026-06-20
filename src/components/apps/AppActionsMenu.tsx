import { ChevronRight, EyeOff, Info, RotateCcw, Trash2 } from 'lucide-react'
import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { horizontalViewportShift } from '../../lib/positioning'
import {
	categoryLabel,
	type AppCategory,
	type AppInfo,
	type CategoryDefinition,
} from '../../types'

interface Props {
	app: AppInfo
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onClose(): void
	onMove(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	isHidden?: boolean
	onHide(id: string): void
	onRestore(id: string): void
}

export function AppActionsMenu({
	app,
	categories,
	categoryOrder,
	onClose,
	onMove,
	onInfo,
	onUninstall,
	isHidden = false,
	onHide,
	onRestore,
}: Props) {
	const [showCategories, setShowCategories] = useState(false)
	const [menuShift, setMenuShift] = useState(0)
	const menuShiftRef = useRef(0)
	const menuRef = useRef<HTMLDivElement>(null)
	useLayoutEffect(() => {
		function keepMenuInViewport() {
			const bounds = menuRef.current?.getBoundingClientRect()
			if (!bounds || (bounds.width === 0 && bounds.height === 0)) return
			const nextShift = horizontalViewportShift(
				bounds.left - menuShiftRef.current,
				bounds.right - menuShiftRef.current,
				window.innerWidth,
			)
			menuShiftRef.current = nextShift
			setMenuShift(nextShift)
		}
		keepMenuInViewport()
		window.addEventListener('resize', keepMenuInViewport)
		return () => window.removeEventListener('resize', keepMenuInViewport)
	}, [showCategories])
	useEffect(() => {
		function keydown(event: KeyboardEvent) {
			if (event.key === 'Escape') onClose()
		}
		function pointerdown(event: PointerEvent) {
			if (!menuRef.current?.contains(event.target as Node)) onClose()
		}
		document.addEventListener('keydown', keydown)
		document.addEventListener('pointerdown', pointerdown)
		return () => {
			document.removeEventListener('keydown', keydown)
			document.removeEventListener('pointerdown', pointerdown)
		}
	}, [onClose])
	return (
		<div
			ref={menuRef}
			style={{ transform: `translateX(${menuShift}px)` }}
			role='menu'
			aria-label={`${app.name} actions`}
			className='absolute left-2 top-11 z-110 w-55 max-w-[calc(100vw-1rem)] rounded-xl border border-white/10 bg-slate-900 p-1.5 text-left shadow-2xl shadow-black/40'
		>
			{!isHidden && (
				<button
					type='button'
					role='menuitem'
					onClick={() => setShowCategories(value => !value)}
					className='flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-200 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-blue-400'
				>
					<ChevronRight
						size={15}
						className={showCategories ? 'rotate-90' : ''}
						aria-hidden='true'
					/>
					Move to category
				</button>
			)}
			{!isHidden && showCategories && (
				<div className='max-h-56 overflow-y-auto border-y border-white/7 py-1'>
					{categoryOrder.map(category => (
						<button
							key={category}
							type='button'
							role='menuitem'
							aria-current={
								category === app.category ? 'true' : undefined
							}
							onClick={() => {
								onMove(app.id, category)
								onClose()
							}}
							className={`flex w-full items-center rounded-lg px-3 py-1.5 text-sm hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-blue-400 ${category === app.category ? 'text-blue-300' : 'text-slate-300'}`}
						>
							{categoryLabel(categories, category)}
						</button>
					))}
				</div>
			)}
			<button
				type='button'
				role='menuitem'
				onClick={() => {
					onInfo(app)
					onClose()
				}}
				className='flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-200 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-blue-400'
			>
				<Info size={15} aria-hidden='true' />
				App info
			</button>
			<button
				type='button'
				role='menuitem'
				onClick={() => {
					if (isHidden) onRestore(app.id)
					else onHide(app.id)
					onClose()
				}}
				className='flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-200 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-blue-400'
			>
				{isHidden ? (
					<RotateCcw size={15} aria-hidden='true' />
				) : (
					<EyeOff size={15} aria-hidden='true' />
				)}
				{isHidden ? 'Restore to catalog' : 'Hide from catalog'}
			</button>
			{!isHidden && app.canUninstall && (
				<button
					type='button'
					role='menuitem'
					onClick={() => {
						onUninstall(app)
						onClose()
					}}
					className='flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-red-300 hover:bg-red-500/10 focus-visible:outline-2 focus-visible:outline-red-400'
				>
					<Trash2 size={15} aria-hidden='true' />
					Uninstall
				</button>
			)}
			{!isHidden && !app.canUninstall && (
				<button
					type='button'
					role='menuitem'
					disabled
					className='flex w-full cursor-not-allowed items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-500'
				>
					<Trash2 size={15} aria-hidden='true' />
					Uninstall unavailable
				</button>
			)}
		</div>
	)
}
