import { useCallback, useRef, type PointerEvent } from 'react'

export function useSpotlight() {
	const rafRef = useRef<number | undefined>(undefined)

	const onPointerMove = useCallback((event: PointerEvent<HTMLElement>) => {
		const element = event.currentTarget
		const clientX = event.clientX
		const clientY = event.clientY
		cancelAnimationFrame(rafRef.current!)
		rafRef.current = requestAnimationFrame(() => {
			const rect = element.getBoundingClientRect()
			element.style.setProperty('--mouse-x', `${clientX - rect.left}px`)
			element.style.setProperty('--mouse-y', `${clientY - rect.top}px`)
		})
	}, [])
	const onPointerEnter = useCallback((event: PointerEvent<HTMLElement>) => {
		event.currentTarget.style.setProperty('--spotlight-opacity', '1')
	}, [])
	const onPointerLeave = useCallback((event: PointerEvent<HTMLElement>) => {
		cancelAnimationFrame(rafRef.current!)
		event.currentTarget.style.setProperty('--spotlight-opacity', '0')
	}, [])
	return { onPointerMove, onPointerEnter, onPointerLeave }
}
