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
