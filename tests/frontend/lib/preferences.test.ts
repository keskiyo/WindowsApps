import { describe, expect, it, vi } from 'vitest'
import {
	DEFAULT_PREFERENCES,
	PREFERENCES_BACKUP_KEY,
	PREFERENCES_KEY,
	normalizePreferences,
	readPreferences,
	writePreferences,
} from '../../../src/lib/preferences'
import { CATEGORY_ORDER } from '../../../src/types'

describe('preferences', () => {
	it('uses complete defaults', () => {
		expect(DEFAULT_PREFERENCES).toMatchObject({
			version: 5,
			categoryOrder: CATEGORY_ORDER,
			favoriteAppIds: [],
			collapsedCategories: [],
			categoryOverrides: {},
			hiddenAppIds: [],
			promotedAppIds: [],
			promotedAppIdentities: [],
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
			version: 5,
			favoriteAppIds: ['codex'],
			collapsedCategories: ['other'],
			categoryOverrides: {},
			hiddenAppIds: [],
			promotedAppIds: [],
			promotedAppIdentities: [],
		})
	})

	it('upgrades v4 and retains unknown root fields for the next write', () => {
		const normalized = normalizePreferences({
			version: 5,
			favoriteAppIds: ['code'],
			experimentalLayout: { density: 'compact' },
		})

		expect(normalized).toMatchObject({
			version: 5,
			favoriteAppIds: ['code'],
			unknownFields: {
				experimentalLayout: { density: 'compact' },
			},
		})
	})

	it('writes retained unknown fields back at the document root', () => {
		const values = new Map<string, string>([
			[
				PREFERENCES_KEY,
				JSON.stringify({
					version: 4,
					favoriteAppIds: ['code'],
					experimentalLayout: { density: 'compact' },
				}),
			],
		])
		const storage = {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => void values.set(key, value),
		} as unknown as Storage

		const preferences = readPreferences(storage)
		expect(
			writePreferences(storage, {
				...preferences,
				favoriteAppIds: ['code', 'editor'],
			}),
		).toBe(true)

		expect(JSON.parse(values.get(PREFERENCES_KEY) ?? '{}')).toMatchObject({
			version: 5,
			favoriteAppIds: ['code', 'editor'],
			experimentalLayout: { density: 'compact' },
		})
		expect(JSON.parse(values.get(PREFERENCES_KEY) ?? '{}')).not.toHaveProperty(
			'unknownFields',
		)
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
			version: 5,
			categories: DEFAULT_PREFERENCES.categories,
			categoryOrder: [
				'browsers',
				'games',
				...CATEGORY_ORDER.filter(
					category => !['browsers', 'games'].includes(category),
				),
			],
			favoriteAppIds: ['code'],
			favoriteAppIdentities: [],
			collapsedCategories: ['games'],
			categoryOverrides: {},
			hiddenAppIds: [],
			hiddenAppIdentities: [],
			promotedAppIds: [],
			promotedAppIdentities: [],
		})
	})

	it('normalizes hidden application ids', () => {
		expect(
			normalizePreferences({ hiddenAppIds: ['code', 'code', '', 42] })
				.hiddenAppIds,
		).toEqual(['code'])
	})

	// The backend cache has always recovered from its `.bak`; this store used to have nothing,
	// so a value that failed to parse silently became "no favorites, nothing hidden".
	it('recovers the previous state when the stored value is corrupt', () => {
		const values = new Map<string, string>()
		const storage = {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => void values.set(key, value),
		} as unknown as Storage

		writePreferences(storage, {
			...DEFAULT_PREFERENCES,
			favoriteAppIds: ['code'],
		})
		writePreferences(storage, {
			...DEFAULT_PREFERENCES,
			favoriteAppIds: ['code', 'editor'],
		})
		// Whatever damages the primary value — a partial write, a hand-edit — must not take the
		// user's choices with it.
		values.set(PREFERENCES_KEY, '{truncated')

		expect(readPreferences(storage).favoriteAppIds).toEqual(['code'])
	})

	it('keeps the backup one step behind the current value', () => {
		const values = new Map<string, string>()
		const storage = {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => void values.set(key, value),
		} as unknown as Storage

		writePreferences(storage, { ...DEFAULT_PREFERENCES, hiddenAppIds: ['a'] })
		writePreferences(storage, { ...DEFAULT_PREFERENCES, hiddenAppIds: ['b'] })

		expect(readPreferences(storage).hiddenAppIds).toEqual(['b'])
		expect(
			normalizePreferences(
				JSON.parse(values.get(PREFERENCES_BACKUP_KEY) ?? '{}'),
			).hiddenAppIds,
		).toEqual(['a'])
	})

	// A newer build may have written a version 6 document; this build (v5) must not overwrite it
	// with the older shape and strip the fields it does not know about.
	it('does not overwrite a document written by a newer version', () => {
		const future = JSON.stringify({
			version: 6,
			favoriteAppIds: ['keep'],
			futureField: 'preserved',
		})
		const values = new Map<string, string>([[PREFERENCES_KEY, future]])
		const storage = {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => void values.set(key, value),
		} as unknown as Storage

		expect(writePreferences(storage, DEFAULT_PREFERENCES)).toBe(true)
		expect(values.get(PREFERENCES_KEY)).toBe(future)
	})

	it('writes normally when the stored version is current or older', () => {
		const values = new Map<string, string>([
			[PREFERENCES_KEY, JSON.stringify({ version: 5, favoriteAppIds: [] })],
		])
		const storage = {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => void values.set(key, value),
		} as unknown as Storage

		writePreferences(storage, {
			...DEFAULT_PREFERENCES,
			favoriteAppIds: ['code'],
		})

		expect(
			JSON.parse(values.get(PREFERENCES_KEY) ?? '{}').favoriteAppIds,
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

	it('writes the versioned document and reports success', () => {
		const storage = { setItem: vi.fn() } as unknown as Storage
		expect(writePreferences(storage, DEFAULT_PREFERENCES)).toBe(true)
		expect(storage.setItem).toHaveBeenCalledWith(
			PREFERENCES_KEY,
			JSON.stringify(DEFAULT_PREFERENCES),
		)
	})

	// A refused write used to be swallowed, so favorites and hidden apps vanished at the next
	// start with nothing told to the user. It must stay non-throwing, but it has to report.
	it('reports a refused write instead of failing silently', () => {
		const throwing = {
			setItem: vi.fn(() => {
				throw new Error('QuotaExceededError')
			}),
		} as unknown as Storage
		expect(() => writePreferences(throwing, DEFAULT_PREFERENCES)).not.toThrow()
		expect(writePreferences(throwing, DEFAULT_PREFERENCES)).toBe(false)
	})
})
