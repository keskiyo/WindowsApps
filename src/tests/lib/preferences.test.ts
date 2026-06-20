import { describe, expect, it, vi } from 'vitest'
import {
	DEFAULT_PREFERENCES,
	PREFERENCES_KEY,
	normalizePreferences,
	readPreferences,
	writePreferences,
} from '../../lib/preferences'
import { CATEGORY_ORDER } from '../../types'

describe('preferences', () => {
	it('uses complete defaults', () => {
		expect(DEFAULT_PREFERENCES).toMatchObject({
			version: 4,
			categoryOrder: CATEGORY_ORDER,
			favoriteAppIds: [],
			collapsedCategories: [],
			categoryOverrides: {},
			hiddenAppIds: [],
		})
		expect(DEFAULT_PREFERENCES.categories).toHaveLength(
			CATEGORY_ORDER.length,
		)
	})

	it('migrates v1 preferences to v2', () => {
		expect(
			normalizePreferences({
				version: 1,
				categoryOrder: ['games'],
				favoriteAppIds: ['codex'],
				collapsedCategories: ['other'],
			}),
		).toMatchObject({
			version: 4,
			favoriteAppIds: ['codex'],
			collapsedCategories: ['other'],
			categoryOverrides: {},
			hiddenAppIds: [],
		})
	})

	it('keeps only valid category overrides', () => {
		expect(
			normalizePreferences({
				version: 2,
				categoryOverrides: {
					codex: 'ai',
					wow: 'games',
					broken: 'missing',
					'': 'games',
				},
			}).categoryOverrides,
		).toEqual({ codex: 'ai', wow: 'games' })
	})

	it('normalizes duplicates and appends missing categories', () => {
		expect(
			normalizePreferences({
				version: 1,
				categoryOrder: ['browsers', 'games', 'browsers', 'invalid'],
				favoriteAppIds: ['code', 'code', 42],
				collapsedCategories: ['games', 'invalid'],
			}),
		).toEqual({
			version: 4,
			categories: DEFAULT_PREFERENCES.categories,
			categoryOrder: [
				'browsers',
				'games',
				...CATEGORY_ORDER.filter(
					category => !['browsers', 'games'].includes(category),
				),
			],
			favoriteAppIds: ['code'],
			collapsedCategories: ['games'],
			categoryOverrides: {},
			hiddenAppIds: [],
		})
	})

	it('normalizes hidden application ids', () => {
		expect(
			normalizePreferences({ hiddenAppIds: ['code', 'code', '', 42] })
				.hiddenAppIds,
		).toEqual(['code'])
	})

	it('falls back when storage is malformed or unavailable', () => {
		const malformed = {
			getItem: vi.fn(() => '{bad json'),
		} as unknown as Storage
		const throwing = {
			getItem: vi.fn(() => {
				throw new Error('denied')
			}),
		} as unknown as Storage
		expect(readPreferences(malformed)).toEqual(DEFAULT_PREFERENCES)
		expect(readPreferences(throwing)).toEqual(DEFAULT_PREFERENCES)
	})

	it('writes the versioned document and ignores write failures', () => {
		const storage = { setItem: vi.fn() } as unknown as Storage
		writePreferences(storage, DEFAULT_PREFERENCES)
		expect(storage.setItem).toHaveBeenCalledWith(
			PREFERENCES_KEY,
			JSON.stringify(DEFAULT_PREFERENCES),
		)
		const throwing = {
			setItem: vi.fn(() => {
				throw new Error('denied')
			}),
		} as unknown as Storage
		expect(() =>
			writePreferences(throwing, DEFAULT_PREFERENCES),
		).not.toThrow()
	})
})
