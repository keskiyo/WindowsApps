export function horizontalViewportShift(
	left: number,
	right: number,
	viewportWidth: number,
	padding = 11,
): number {
	if (right > viewportWidth - padding) return viewportWidth - padding - right
	if (left < padding) return padding - left
	return 0
}

interface AnchorBounds {
	left: number
	right: number
	top: number
	bottom: number
}

export function floatingMenuPosition(
	anchor: AnchorBounds,
	menuWidth: number,
	menuHeight: number,
	viewportWidth: number,
	viewportHeight: number,
	padding = 12,
	gap = 4,
): { left: number; top: number } {
	const left = Math.min(
		Math.max(anchor.left, padding),
		Math.max(padding, viewportWidth - menuWidth - padding),
	)
	const top = Math.min(
		Math.max(anchor.bottom + gap, padding),
		Math.max(padding, viewportHeight - menuHeight - padding),
	)
	return { left, top }
}

export function requiredMenuScroll(
	anchorBottom: number,
	menuHeight: number,
	viewportHeight: number,
	padding = 12,
	gap = 4,
): number {
	return Math.max(
		0,
		anchorBottom + gap + menuHeight - (viewportHeight - padding),
	)
}
