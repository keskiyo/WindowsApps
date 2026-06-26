import { describe, expect, it } from 'vitest'
import { catalogChangeMessage } from '../../lib/catalogChanges'

describe('catalog change messages', () => {
	it('formats added and removed application counts', () => {
		expect(catalogChangeMessage({ added: 1, removed: 0, updated: 0 })).toBe(
			'1 application added',
		)
		expect(catalogChangeMessage({ added: 2, removed: 0, updated: 0 })).toBe(
			'2 applications added',
		)
		expect(catalogChangeMessage({ added: 0, removed: 1, updated: 0 })).toBe(
			'1 application removed',
		)
	})

	it('uses a generic message for mixed changes and suppresses empty updates', () => {
		expect(catalogChangeMessage({ added: 1, removed: 1, updated: 0 })).toBe(
			'Application catalog updated',
		)
		expect(
			catalogChangeMessage({ added: 0, removed: 0, updated: 0 }),
		).toBeNull()
	})
})
