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
	floatingSubmenuPosition,
	requiredMenuScroll,
} from '../../../shared/lib/positioning'

interface Options {
	anchorRef: RefObject<HTMLButtonElement | null>
	onClose(): void
	showCategories: boolean
}

export function useActionsMenu({
	anchorRef,
	onClose,
	showCategories,
}: Options) {
	const [position, setPosition] = useState({ left: 12, top: 48 })
	const [categoryPosition, setCategoryPosition] = useState({
		left: 12,
		top: 48,
	})
	const menuRef = useRef<HTMLDivElement>(null)
	const categoryMenuRef = useRef<HTMLDivElement>(null)
	const adjustedHeightRef = useRef(0)

	useLayoutEffect(() => {
		function placeMenu() {
			const anchor = anchorRef.current?.getBoundingClientRect()
			const menu = menuRef.current?.getBoundingClientRect()
			if (!anchor || !menu || (menu.width === 0 && menu.height === 0))
				return
			const nextPosition = floatingMenuPosition(
				anchor,
				menu.width,
				menu.height,
				window.innerWidth,
				window.innerHeight,
			)
			setPosition(nextPosition)
			const nextMenuBounds = {
				left: nextPosition.left,
				right: nextPosition.left + menu.width,
				top: nextPosition.top,
				bottom: nextPosition.top + menu.height,
			}
			const categoryMenu =
				categoryMenuRef.current?.getBoundingClientRect()
			if (
				showCategories &&
				categoryMenu &&
				(categoryMenu.width !== 0 || categoryMenu.height !== 0)
			) {
				setCategoryPosition(
					floatingSubmenuPosition(
						nextMenuBounds,
						categoryMenu.width,
						categoryMenu.height,
						window.innerWidth,
						window.innerHeight,
					),
				)
			}
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
			if (menuRef.current?.contains(target)) return
			if (categoryMenuRef.current?.contains(target)) return
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

	useEffect(() => {
		const first = menuRef.current?.querySelector<HTMLElement>(
			'[role="menuitem"]:not([disabled])',
		)
		first?.focus()
	}, [])

	function onMenuKeyDown(event: ReactKeyboardEvent) {
		if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
		const menuItems = Array.from(
			menuRef.current?.querySelectorAll<HTMLElement>(
				'[role="menuitem"]:not([disabled])',
			) ?? [],
		)
		const categoryItems = Array.from(
			categoryMenuRef.current?.querySelectorAll<HTMLElement>(
				'[role="menuitem"]:not([disabled])',
			) ?? [],
		)
		const items = categoryItems.length
			? [menuItems[0]!, ...categoryItems, ...menuItems.slice(1)]
			: menuItems
		if (items.length === 0) return
		event.preventDefault()
		const current = items.indexOf(document.activeElement as HTMLElement)
		const delta = event.key === 'ArrowDown' ? 1 : -1
		const next = (current + delta + items.length) % items.length
		items[next].focus()
	}

	return {
		menuRef,
		categoryMenuRef,
		position,
		categoryPosition,
		onMenuKeyDown,
	}
}
