import { useEffect } from 'react'

/**
 * The shell scrolls its own panel, not the document: the window is a fixed-height flex layout, so
 * `document.body` never overflows and hiding its overflow locked nothing — a dialog's backdrop
 * still scrolled the catalog behind it. Both are locked, the panel by id because it is the only
 * scroll container outside a dialog and giving `shared` a selector is cheaper than threading a ref
 * through every layer between the shell and a modal.
 */
const SCROLL_ROOT = 'catalog-scroll'

// Module-level ref counter so nested modals don't fight over the lock.
// First lock hides scroll; last unlock restores it.
let lockCount = 0

function setScrollLock(locked: boolean) {
	const value = locked ? 'hidden' : ''
	document.body.style.overflow = value
	// Re-queried on release rather than captured: a view switch can replace the panel while a
	// dialog is open, and restoring overflow on a detached node would leave the live one locked.
	const root = document.getElementById(SCROLL_ROOT)
	if (root) root.style.overflowY = value
}

export function useBodyScrollLock() {
	useEffect(() => {
		if (++lockCount === 1) setScrollLock(true)
		return () => {
			if (--lockCount === 0) setScrollLock(false)
		}
	}, [])
}
