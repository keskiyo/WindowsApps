import { useEffect } from 'react'

// Module-level ref counter so nested modals don't fight over body.overflow.
// First lock hides scroll; last unlock restores it.
let lockCount = 0

export function useBodyScrollLock() {
	useEffect(() => {
		if (++lockCount === 1) document.body.style.overflow = 'hidden'
		return () => {
			if (--lockCount === 0) document.body.style.overflow = ''
		}
	}, [])
}
