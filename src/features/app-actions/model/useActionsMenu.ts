import {
	useEffect,
	useLayoutEffect,
	useRef,
	useState,
	type KeyboardEvent as ReactKeyboardEvent,
	type RefObject,
} from 'react'
import {
	floatingMenuPosition,
	requiredMenuScroll,
} from '../../../shared/lib/positioning'

interface Options {
	anchorRef: RefObject<HTMLButtonElement | null>
	onClose(): void
	showCategories: boolean
}

/**
 * Owns the floating menu's placement, dismissal, and keyboard behavior so the component stays
 * presentational. Returns the menu ref, computed position, and the arrow-key handler.
 */
export function useActionsMenu({
	anchorRef,
	onClose,
	showCategories,
}: Options) {
	const [position, setPosition] = useState({ left: 12, top: 48 })
	const menuRef = useRef<HTMLDivElement>(null)
	const adjustedHeightRef = useRef(0)

	useLayoutEffect(() => {
		function placeMenu() {
			const anchor = anchorRef.current?.getBoundingClientRect()
			const menu = menuRef.current?.getBoundingClientRect()
			if (!anchor || !menu || (menu.width === 0 && menu.height === 0))
				return
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
				? Math.max(
						0,
						catalog.scrollHeight -
							catalog.clientHeight -
							catalog.scrollTop,
					)
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
			const target = event.target as Node
			// Ignore clicks on the anchor (grip) button: it owns the open/close toggle. Without
			// this, a click on the grip while open fires pointerdown (closing) then click
			// (re-toggling open), so the menu never closes on a repeat press.
			if (menuRef.current?.contains(target)) return
			if (anchorRef.current?.contains(target)) return
			onClose()
		}
		document.addEventListener('keydown', keydown)
		document.addEventListener('pointerdown', pointerdown)
		return () => {
			document.removeEventListener('keydown', keydown)
			document.removeEventListener('pointerdown', pointerdown)
		}
	}, [onClose, anchorRef])

	// WAI-ARIA menu pattern: move focus into the menu on open so arrow keys work immediately.
	useEffect(() => {
		const first = menuRef.current?.querySelector<HTMLElement>(
			'[role="menuitem"]:not([disabled])',
		)
		first?.focus()
	}, [])

	function onMenuKeyDown(event: ReactKeyboardEvent) {
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

	return { menuRef, position, onMenuKeyDown }
}
