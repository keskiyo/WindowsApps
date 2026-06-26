import { useEffect, type RefObject } from 'react'

const FOCUSABLE =
	'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])'

/**
 * Traps Tab / Shift+Tab focus within the referenced container while it is mounted, so a
 * modal can't leak focus to the page behind it (WCAG 2.1 — No Keyboard Trap inverse: keep
 * focus inside an explicitly modal surface). Pair with `aria-modal='true'` on the element.
 */
export function useFocusTrap(ref: RefObject<HTMLElement | null>) {
	useEffect(() => {
		const container = ref.current
		if (!container) return
		function onKeyDown(event: KeyboardEvent) {
			if (event.key !== 'Tab' || !container) return
			const items = Array.from(
				container.querySelectorAll<HTMLElement>(FOCUSABLE),
			).filter(
				el => el.offsetParent !== null || el === document.activeElement,
			)
			if (items.length === 0) return
			const first = items[0]
			const last = items[items.length - 1]
			const active = document.activeElement
			if (event.shiftKey) {
				if (active === first || !container.contains(active)) {
					event.preventDefault()
					last.focus()
				}
			} else if (active === last) {
				event.preventDefault()
				first.focus()
			}
		}
		document.addEventListener('keydown', onKeyDown)
		return () => document.removeEventListener('keydown', onKeyDown)
	}, [ref])
}
