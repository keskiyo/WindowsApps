import { ChevronRight, EyeOff, Info, RotateCcw, Trash2, Wrench } from 'lucide-react'
import {
	useEffect,
	useLayoutEffect,
	useRef,
	useState,
	type CSSProperties,
	type RefObject,
} from 'react'
import { createPortal } from 'react-dom'
import { useSpotlight } from '../../hooks/useSpotlight'
import {
	floatingMenuPosition,
	requiredMenuScroll,
} from '../../lib/positioning'
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
	isUserPromoted?: boolean
	onHide(id: string): void
	onRestore(id: string): void
	onDemote(id: string): void
	anchorRef: RefObject<HTMLButtonElement | null>
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
	isUserPromoted = false,
	onHide,
	onRestore,
	onDemote,
	anchorRef,
}: Props) {
	const spotlight = useSpotlight()
	const [showCategories, setShowCategories] = useState(false)
	const [position, setPosition] = useState({ left: 12, top: 48 })
	const menuRef = useRef<HTMLDivElement>(null)
	const adjustedHeightRef = useRef(0)
	useLayoutEffect(() => {
		function placeMenu() {
			const anchor = anchorRef.current?.getBoundingClientRect()
			const menu = menuRef.current?.getBoundingClientRect()
			if (!anchor || !menu || (menu.width === 0 && menu.height === 0)) return
			setPosition(
				floatingMenuPosition(
					anchor,
					menu.width,
					menu.height,
					window.innerWidth,
					window.innerHeight,
				),
			)
			const menuHeight = Math.round(menu.height)
			const scrollAmount = requiredMenuScroll(
				anchor.bottom,
				menu.height,
				window.innerHeight,
			)
			const catalog = document.getElementById('catalog-scroll')
			const remaining = catalog
				? Math.max(0, catalog.scrollHeight - catalog.clientHeight - catalog.scrollTop)
				: 0
			if (
				catalog &&
				scrollAmount > 0 &&
				remaining > 0 &&
				adjustedHeightRef.current !== menuHeight
			) {
				adjustedHeightRef.current = menuHeight
				catalog.scrollBy({
					top: Math.min(scrollAmount, remaining),
					behavior: 'smooth',
				})
			}
		}
		placeMenu()
		window.addEventListener('resize', placeMenu)
		window.addEventListener('scroll', placeMenu, true)
		return () => {
			window.removeEventListener('resize', placeMenu)
			window.removeEventListener('scroll', placeMenu, true)
		}
	}, [anchorRef, showCategories])
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
	// WAI-ARIA menu pattern: move focus into the menu on open so arrow keys work immediately.
	useEffect(() => {
		const first = menuRef.current?.querySelector<HTMLElement>(
			'[role="menuitem"]:not([disabled])',
		)
		first?.focus()
	}, [])
	function onMenuKeyDown(event: React.KeyboardEvent) {
		if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
		const items = Array.from(
			menuRef.current?.querySelectorAll<HTMLElement>(
				'[role="menuitem"]:not([disabled])',
			) ?? [],
		)
		if (items.length === 0) return
		event.preventDefault()
		const current = items.indexOf(document.activeElement as HTMLElement)
		const delta = event.key === 'ArrowDown' ? 1 : -1
		const next = (current + delta + items.length) % items.length
		items[next].focus()
	}
	return createPortal(
		<div
			ref={menuRef}
			onKeyDown={onMenuKeyDown}
			style={
				{
					left: position.left,
					top: position.top,
					// Reset the spotlight vars so menu items don't inherit the parent card's
					// glow (each item drives its own on hover).
					'--spotlight-opacity': 0,
				} as CSSProperties
			}
			role='menu'
			aria-label={`${app.name} actions`}
			className='motion-panel fixed z-[600] flex max-h-[calc(100vh-1.5rem)] w-56 max-w-[calc(100vw-1.5rem)] flex-col gap-0.5 overflow-y-auto rounded-xl border border-slate-200/85 bg-slate-50 p-2 text-left text-slate-700 shadow-[0_18px_45px_rgba(53,61,82,.2)]'
		>
			{!isHidden && (
				<button
					type='button'
					role='menuitem'
					onClick={() => setShowCategories(value => !value)}
					{...spotlight}
					className='relative flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm text-slate-700 hover:bg-slate-500/15 focus-visible:outline-2 focus-visible:outline-violet-500'
				>
					<SpotlightLayer size={70} />
					<ChevronRight
						size={15}
						className={`text-slate-400 transition-transform ${showCategories ? 'rotate-90' : ''}`}
						aria-hidden='true'
					/>
					Move to category
				</button>
			)}
			{!isHidden && showCategories && (
				<div className='my-1 flex max-h-56 flex-col gap-0.5 overflow-y-auto overscroll-contain rounded-lg bg-slate-500/8 p-1'>
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
							className={`relative flex w-full items-center rounded-lg px-3 py-2 text-sm focus-visible:outline-2 focus-visible:outline-violet-500 ${category === app.category ? 'bg-violet-500/18 font-medium text-violet-300' : 'text-slate-600 hover:bg-slate-500/15'}`}
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
				className='relative flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm text-slate-700 hover:bg-slate-500/15 focus-visible:outline-2 focus-visible:outline-violet-500'
			>
				<SpotlightLayer size={70} />
				<Info size={15} className='text-slate-400' aria-hidden='true' />
				App info
			</button>
			<button
				type='button'
				role='menuitem'
				onClick={() => {
					if (isHidden) onRestore(app.id)
					else if (isUserPromoted) onDemote(app.id)
					else onHide(app.id)
					onClose()
				}}
				{...spotlight}
				className='relative flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm text-slate-700 hover:bg-slate-500/15 focus-visible:outline-2 focus-visible:outline-violet-500'
			>
				<SpotlightLayer size={70} />
				{isHidden ? (
					<RotateCcw size={15} className='text-slate-400' aria-hidden='true' />
				) : isUserPromoted ? (
					<Wrench size={15} className='text-slate-400' aria-hidden='true' />
				) : (
					<EyeOff size={15} className='text-slate-400' aria-hidden='true' />
				)}
				{isHidden
					? 'Restore to catalog'
					: isUserPromoted
						? 'Move back to Auxiliary tools'
						: 'Hide from catalog'}
			</button>
			{!isHidden && (
				<div className='mx-1 my-1 border-t border-slate-200/55' />
			)}
			{!isHidden && app.canUninstall && (
				<button
					type='button'
					role='menuitem'
					onClick={() => {
						onUninstall(app)
						onClose()
					}}
					className='flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm text-rose-300 hover:bg-rose-400/15 hover:text-rose-200 focus-visible:outline-2 focus-visible:outline-rose-300/70'
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
					className='flex w-full cursor-not-allowed items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm text-slate-500'
				>
					<Trash2 size={15} aria-hidden='true' />
					Uninstall unavailable
				</button>
			)}
		</div>,
		document.querySelector<HTMLElement>('.app-shell') ?? document.body,
	)
}
