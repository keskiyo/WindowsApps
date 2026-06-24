import type { CSSProperties } from 'react'

/**
 * Neon flashlight border layer (design/design.md). Drop as the first child of any
 * `position: relative` element that also spreads the `useSpotlight` pointer handlers.
 * `size` controls the lit radius (px); the ring inherits the parent's border-radius.
 */
export function SpotlightLayer({ size }: { size?: number }) {
	return (
		<span
			aria-hidden='true'
			className='spotlight'
			style={
				size
					? ({ '--spotlight-size': `${size}px` } as CSSProperties)
					: undefined
			}
		/>
	)
}
