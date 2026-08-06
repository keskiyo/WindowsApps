import type { CSSProperties } from 'react'

export function SpotlightLayer({ size }: { size?: number }) {
	return (
		<span
			aria-hidden="true"
			className="spotlight"
			style={
				size
					? ({ '--spotlight-size': `${size}px` } as CSSProperties)
					: undefined
			}
		/>
	)
}
