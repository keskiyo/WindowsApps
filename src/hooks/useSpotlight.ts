import { useCallback, useRef, type PointerEvent } from 'react'

/**
 * Neon "flashlight" border effect (see design/design.md). Writes the cursor position and
 * fade into CSS custom properties directly on the hovered element — no React state, so it
 * never re-renders the card and stays compatible with `React.memo`. Pair with the
 * `.card-spotlight` layer in index.css, which reads `--mouse-x`/`--mouse-y`/`--spotlight-opacity`.
 */
export function useSpotlight() {
	// Batch pointer-move updates through rAF to avoid forced layout reflows on every event.
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
