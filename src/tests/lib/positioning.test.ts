import { describe, expect, it } from 'vitest'
import {
	floatingMenuPosition,
	horizontalViewportShift,
	requiredMenuScroll,
} from '../../lib/positioning'

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

describe('requiredMenuScroll', () => {
	it('does not move the catalog when the menu already fits', () => {
		expect(requiredMenuScroll(300, 240, 720)).toBe(0)
	})

	it('returns only the pixels needed to reveal the menu', () => {
		expect(requiredMenuScroll(620, 180, 720)).toBe(96)
	})
})

describe('floatingMenuPosition', () => {
	const anchor = {
		left: 900,
		right: 932,
		top: 220,
		bottom: 252,
	}

	it('keeps a right-edge menu inside the viewport', () => {
		expect(floatingMenuPosition(anchor, 224, 300, 1080, 720)).toEqual({
			left: 844,
			top: 256,
		})
	})

	it('keeps the menu within the viewport without flipping above the trigger', () => {
		expect(
			floatingMenuPosition(
				{ left: 200, right: 232, top: 620, bottom: 652 },
				224,
				300,
				1080,
				720,
			),
		).toEqual({ left: 200, top: 408 })
	})
})
