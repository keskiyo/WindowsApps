import { useEffect } from 'react'

const SCROLL_ROOT = 'catalog-scroll'

let lockCount = 0

function setScrollLock(locked: boolean) {
	const value = locked ? 'hidden' : ''
	document.body.style.overflow = value
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
