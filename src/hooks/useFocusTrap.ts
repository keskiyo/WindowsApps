import { useEffect, type RefObject } from 'react'

const FOCUSABLE =
	'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])'

/**
 * Whether an element can actually receive focus.
 *
 * This deliberately does not consult layout. The previous check was
 * `offsetParent !== null || el === document.activeElement`, and jsdom never computes layout, so
 * under test the set collapsed to the single focused element: `first === last`, and the trap
 * silently stopped trapping. Every dialog using this hook therefore had an unverifiable focus
 * trap. Attribute-based reachability behaves the same in both environments, and the dialogs
 * here render conditionally rather than hiding focusables with CSS.
 */
function isReachable(element: HTMLElement): boolean {
	return (
		!element.hasAttribute('hidden') &&
		element.getAttribute('aria-hidden') !== 'true' &&
		!(element as HTMLButtonElement).disabled
	)
}

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
			).filter(isReachable)
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
