import { useCallback, type PointerEvent } from 'react'

/**
 * Neon "flashlight" border effect (see design/design.md). Writes the cursor position and
 * fade into CSS custom properties directly on the hovered element — no React state, so it
 * never re-renders the card and stays compatible with `React.memo`. Pair with the
 * `.card-spotlight` layer in index.css, which reads `--mouse-x`/`--mouse-y`/`--spotlight-opacity`.
 */
export function useSpotlight() {
	const onPointerMove = useCallback(
		(event: PointerEvent<HTMLElement>) => {
			const element = event.currentTarget
			const rect = element.getBoundingClientRect()
			element.style.setProperty('--mouse-x', `${event.clientX - rect.left}px`)
			element.style.setProperty('--mouse-y', `${event.clientY - rect.top}px`)
		},
		[],
	)
	const onPointerEnter = useCallback(
		(event: PointerEvent<HTMLElement>) => {
			event.currentTarget.style.setProperty('--spotlight-opacity', '1')
		},
		[],
	)
	const onPointerLeave = useCallback(
		(event: PointerEvent<HTMLElement>) => {
			event.currentTarget.style.setProperty('--spotlight-opacity', '0')
		},
		[],
	)
	return { onPointerMove, onPointerEnter, onPointerLeave }
}
