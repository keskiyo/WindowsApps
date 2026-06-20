import { describe, expect, it } from 'vitest'
import { horizontalViewportShift } from '../../lib/positioning'

describe('horizontalViewportShift', () => {
	it('moves an overflowing menu left into the viewport', () => {
		expect(horizontalViewportShift(185, 405, 342)).toBe(-74)
	})

	it('moves an overflowing menu right into the viewport', () => {
		expect(horizontalViewportShift(-30, 190, 342)).toBe(41)
	})

	it('keeps a visible menu in place', () => {
		expect(horizontalViewportShift(40, 260, 342)).toBe(0)
	})
})
