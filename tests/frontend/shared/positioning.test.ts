import { describe, expect, it } from 'vitest'
import {
	floatingMenuPosition,
	floatingSubmenuPosition,
	requiredMenuScroll,
} from '../../../src/shared/lib/positioning'

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

describe('floatingSubmenuPosition', () => {
	it('positions the panel to the right when it fits after viewport padding', () => {
		expect(
			floatingSubmenuPosition(
				{ left: 120, right: 344, top: 100, bottom: 300 },
				200,
				240,
				1024,
				768,
			),
		).toMatchObject({ left: 348, top: 100, side: 'right' })
	})

	it('falls back left and clamps the panel inside viewport padding', () => {
		expect(
			floatingSubmenuPosition(
				{ left: 740, right: 964, top: 600, bottom: 760 },
				200,
				240,
				1024,
				768,
			),
		).toMatchObject({ left: 536, top: 516, side: 'left' })
	})
})
