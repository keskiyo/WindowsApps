import { describe, expect, it } from 'vitest'
import { getDropAction } from '../../../lib/catalog'

describe('application drop routing', () => {
	it('moves an application to the target category', () => {
		expect(
			getDropAction(
				{ type: 'app', appId: 'wow', category: 'other' },
				{ type: 'category', category: 'games' },
			),
		).toEqual({ type: 'move-app', appId: 'wow', category: 'games' })
	})

	it('reorders categories independently from application drops', () => {
		expect(
			getDropAction(
				{ type: 'category-sort', category: 'games' },
				{ type: 'category-sort', category: 'ai' },
			),
		).toEqual({ type: 'reorder-category', active: 'games', over: 'ai' })
	})

	it('reorders a category over the nested category drop target', () => {
		expect(
			getDropAction(
				{ type: 'category-sort', category: 'games' },
				{ type: 'category', category: 'ai' },
			),
		).toEqual({ type: 'reorder-category', active: 'games', over: 'ai' })
	})
})
