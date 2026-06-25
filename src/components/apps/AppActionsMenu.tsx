import { ChevronRight, EyeOff, Info, RotateCcw, Trash2 } from 'lucide-react'
import {
	useEffect,
	useLayoutEffect,
	useRef,
	useState,
	type CSSProperties,
} from 'react'
import { useSpotlight } from '../../hooks/useSpotlight'
import { horizontalViewportShift } from '../../lib/positioning'
import { SpotlightLayer } from '../shared/SpotlightLayer'
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
	const spotlight = useSpotlight()
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
	// When the menu (or its expanded category list) doesn't fit the viewport, gently
	// scroll it fully into view instead of leaving it clipped at the edge.
	useEffect(() => {
		const element = menuRef.current
		if (!element) return
		const id = requestAnimationFrame(() => {
			const bounds = element.getBoundingClientRect()
			if (bounds.bottom > window.innerHeight - 8 || bounds.top < 44) {
				element.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
			}
		})
		return () => cancelAnimationFrame(id)
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
			style={
				{
					transform: `translateX(${menuShift}px)`,
					// Reset the spotlight vars so menu items don't inherit the parent card's
					// glow (each item drives its own on hover).
					'--spotlight-opacity': 0,
				} as CSSProperties
			}
			role='menu'
			aria-label={`${app.name} actions`}
			className='absolute left-2 top-11 z-110 w-55 max-w-[calc(100vw-1rem)] rounded-xl border border-slate-200/85 bg-slate-50 p-1.5 text-left text-slate-700 shadow-[0_18px_45px_rgba(53,61,82,.2)]'
		>
			{!isHidden && (
				<button
					type='button'
					role='menuitem'
					onClick={() => setShowCategories(value => !value)}
					{...spotlight}
					className='relative flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-700 hover:bg-violet-100/70 focus-visible:outline-2 focus-visible:outline-violet-500'
				>
					<SpotlightLayer size={70} />
					<ChevronRight
						size={15}
						className={showCategories ? 'rotate-90' : ''}
						aria-hidden='true'
					/>
					Move to category
				</button>
			)}
			{!isHidden && showCategories && (
				<div className='max-h-56 overflow-y-auto overscroll-contain border-y border-slate-200 py-1'>
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
							{...spotlight}
							className={`relative flex w-full items-center rounded-lg px-3 py-1.5 text-sm hover:bg-violet-100/70 focus-visible:outline-2 focus-visible:outline-violet-500 ${category === app.category ? 'font-medium text-violet-700' : 'text-slate-600'}`}
						>
							<SpotlightLayer size={60} />
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
				{...spotlight}
				className='relative flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-700 hover:bg-violet-100/70 focus-visible:outline-2 focus-visible:outline-violet-500'
			>
				<SpotlightLayer size={70} />
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
				{...spotlight}
				className='relative flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-slate-700 hover:bg-violet-100/70 focus-visible:outline-2 focus-visible:outline-violet-500'
			>
				<SpotlightLayer size={70} />
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
					className='flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-red-700 hover:bg-red-100 focus-visible:outline-2 focus-visible:outline-red-500'
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
