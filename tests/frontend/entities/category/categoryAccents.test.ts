import { describe, expect, it } from 'vitest'
import {
	chooseCustomCategoryAccent,
	stableCustomCategoryAccent,
} from '../../../../src/entities/category/lib/categoryAccents'

describe('category accents', () => {
	it('uses an available accent before repeating one', () => {
		expect(
			chooseCustomCategoryAccent(['yellow', 'cyan'], () => 0),
		).toBe('pink')
	})

	it('derives the same accent for a migrated category id', () => {
		expect(stableCustomCategoryAccent('custom:work')).toBe(
			stableCustomCategoryAccent('custom:work'),
		)
	})
})
