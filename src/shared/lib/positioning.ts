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

export function floatingSubmenuPosition(
	menu: AnchorBounds,
	panelWidth: number,
	panelHeight: number,
	viewportWidth: number,
	viewportHeight: number,
	padding = 12,
	gap = 4,
): { left: number; top: number; side: 'left' | 'right' } {
	const fitsRight = menu.right + gap + panelWidth <= viewportWidth - padding
	const left = fitsRight ? menu.right + gap : menu.left - gap - panelWidth

	return {
		left: Math.min(
			Math.max(left, padding),
			Math.max(padding, viewportWidth - panelWidth - padding),
		),
		top: Math.min(
			Math.max(menu.top, padding),
			Math.max(padding, viewportHeight - panelHeight - padding),
		),
		side: fitsRight ? 'right' : 'left',
	}
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
