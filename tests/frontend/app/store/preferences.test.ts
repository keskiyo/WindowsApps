import { describe, expect, it, vi } from 'vitest'
import {
	DEFAULT_PREFERENCES,
	PREFERENCES_BACKUP_KEY,
	PREFERENCES_KEY,
	normalizePreferences,
	readPreferences,
	writePreferences,
} from '../../../../src/app/store/preferences'
import { stableCustomCategoryAccent } from '../../../../src/entities/category/lib/categoryAccents'
import { CATEGORY_ORDER } from '../../../../src/entities/category'

describe('preferences', () => {
	it('uses complete defaults', () => {
		expect(DEFAULT_PREFERENCES).toMatchObject({
			version: 14,
			favoriteScenarioIds: [],
			categoryOrder: CATEGORY_ORDER,
			favoriteAppIds: [],
			collapsedCategories: [],
			categoryOverrides: {},
			categoryOverrideIdentities: {},
			hiddenAppIds: [],
			promotedAppIds: [],
			promotedAppIdentities: [],
		})
		expect(DEFAULT_PREFERENCES.categories).toHaveLength(
			CATEGORY_ORDER.length,
		)
	})

	it('appends the reserved Installers & Docs category to saved preferences', () => {
		const normalized = normalizePreferences({
			version: 8,
			categories: [{ id: 'games', label: 'Games', builtIn: true }],
			categoryOrder: ['games'],
		})

		expect(normalized.categories).toContainEqual({
			id: 'installers_docs',
			label: 'Installers & Docs',
			builtIn: true,
		})
		expect(normalized.categoryOrder).toContain('installers_docs')
	})

	it('migrates v7 custom categories with stable accents and durable identities', () => {
		const migrated = normalizePreferences({
			version: 7,
			categories: [
				{ id: 'custom:work', label: 'Work', builtIn: false },
				{
					id: 'custom:personal',
					label: 'Personal',
					builtIn: false,
					accent: 'red',
				},
			],
			favoriteAppIdentities: ['identity:codex'],
		})

		expect(migrated).toMatchObject({
			version: 14,
			favoriteAppIdentities: ['identity:codex'],
		})
		expect(migrated.categories).toContainEqual(
			expect.objectContaining({
				id: 'custom:work',
				accent: stableCustomCategoryAccent('custom:work'),
			}),
		)
		expect(migrated.categories).toContainEqual(
			expect.objectContaining({ id: 'custom:personal', accent: 'red' }),
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
			version: 14,
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
			version: 14,
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
			version: 14,
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

	it('migrates a v5 document by defaulting the durable override identity map', () => {
		const normalized = normalizePreferences({
			version: 5,
			categoryOverrides: { codex: 'ai' },
		})
		// The id-keyed map is preserved (the store folds it into identities on the next catalog
		// load); the new identity map defaults to empty so nothing is lost on upgrade.
		expect(normalized.version).toBe(14)
		expect(normalized.categoryOverrides).toEqual({ codex: 'ai' })
		expect(normalized.categoryOverrideIdentities).toEqual({})
	})

	it('quarantines v6 canonical identities for collision-safe catalog reconciliation', () => {
		const normalized = normalizePreferences({
			version: 6,
			favoriteAppIds: ['cmd-shortcut'],
			favoriteAppIdentities: ['product:command-prompt'],
			hiddenAppIds: ['cmd-shortcut'],
			hiddenAppIdentities: ['product:command-prompt'],
			promotedAppIds: ['cmd-shortcut'],
			promotedAppIdentities: ['product:command-prompt'],
			categoryOverrides: { 'cmd-shortcut': 'utilities' },
			categoryOverrideIdentities: {
				'product:command-prompt': 'utilities',
			},
		})

		expect(normalized).toMatchObject({
			version: 14,
			favoriteAppIds: ['cmd-shortcut'],
			favoriteAppIdentities: [],
			hiddenAppIds: ['cmd-shortcut'],
			hiddenAppIdentities: [],
			promotedAppIds: ['cmd-shortcut'],
			promotedAppIdentities: [],
			categoryOverrides: { 'cmd-shortcut': 'utilities' },
			categoryOverrideIdentities: {},
			legacyCanonicalPreferences: {
				favorite: ['product:command-prompt'],
				hidden: ['product:command-prompt'],
				promoted: ['product:command-prompt'],
				categoryOverrides: {
					'product:command-prompt': 'utilities',
				},
			},
		})
	})

	it('keeps v7 card preference identities out of the legacy quarantine', () => {
		const normalized = normalizePreferences({
			version: 8,
			favoriteAppIdentities: ['preference:cmd-shortcut'],
			legacyCanonicalPreferences: {
				favorite: ['product:unresolved'],
			},
		})

		expect(normalized.favoriteAppIdentities).toEqual([
			'preference:cmd-shortcut',
		])
		expect(normalized.legacyCanonicalPreferences.favorite).toEqual([
			'product:unresolved',
		])
	})

	it('upgrades a v8 document to empty manual installer marks without touching the rest', () => {
		const normalized = normalizePreferences({
			version: 8,
			favoriteAppIds: ['code'],
			favoriteAppIdentities: ['preference:code'],
			hiddenAppIdentities: ['preference:helper'],
			promotedAppIdentities: ['preference:tool'],
			categoryOverrideIdentities: { 'preference:code': 'utilities' },
		})

		// v8 could not carry the marks, so the upgrade must default them rather than invent any,
		// and every field the older document did carry has to survive untouched.
		expect(normalized.version).toBe(14)
		expect(normalized.installerAppIds).toEqual([])
		expect(normalized.installerAppIdentities).toEqual([])
		expect(normalized.legacyCanonicalPreferences.installer).toEqual([])
		expect(normalized.favoriteAppIdentities).toEqual(['preference:code'])
		expect(normalized.hiddenAppIdentities).toEqual(['preference:helper'])
		expect(normalized.promotedAppIdentities).toEqual(['preference:tool'])
		expect(normalized.categoryOverrideIdentities).toEqual({
			'preference:code': 'utilities',
		})
	})

	it('reads back the manual installer marks a v9 document carries', () => {
		const normalized = normalizePreferences({
			version: 9,
			installerAppIds: ['setup', 'setup', '', 42],
			installerAppIdentities: ['preference:setup'],
			legacyCanonicalPreferences: { installer: ['product:unresolved'] },
		})

		expect(normalized.installerAppIds).toEqual(['setup'])
		expect(normalized.installerAppIdentities).toEqual(['preference:setup'])
		expect(normalized.legacyCanonicalPreferences.installer).toEqual([
			'product:unresolved',
		])
	})

	it('upgrades a v9 document to empty first-seen stamps', () => {
		const normalized = normalizePreferences({
			version: 9,
			installerAppIdentities: ['preference:setup'],
			firstSeenAt: { 'preference:setup': 1 },
		})

		// v9 could not carry stamps, so a value under that key is not ours to trust; the store
		// refills the map from the next catalog load.
		expect(normalized.version).toBe(14)
		expect(normalized.firstSeenAt).toEqual({})
		expect(normalized.installerAppIdentities).toEqual(['preference:setup'])
	})

	it('keeps only usable first-seen stamps from a v10 document', () => {
		const normalized = normalizePreferences({
			version: 10,
			firstSeenAt: {
				'preference:code': 1700000000000,
				'preference:broken': 'yesterday',
				'preference:zero': 0,
				'preference:infinite': Number.POSITIVE_INFINITY,
				'': 1700000000000,
			},
		})

		expect(normalized.firstSeenAt).toEqual({
			'preference:code': 1700000000000,
		})
	})

	it('upgrades a v10 document to no scenarios and keeps its stamps', () => {
		const normalized = normalizePreferences({
			version: 10,
			firstSeenAt: { 'preference:code': 1700000000000 },
			scenarios: [{ id: 's1', name: 'Work', launchIdentities: ['x'] }],
		})

		// v10 could not carry scenarios, so a value under that key is not ours to trust.
		expect(normalized.version).toBe(14)
		expect(normalized.scenarios).toEqual([])
		expect(normalized.firstSeenAt).toEqual({
			'preference:code': 1700000000000,
		})
	})

	it('keeps only scenarios a stored document can act on', () => {
		const normalized = normalizePreferences({
			version: 12,
			scenarios: [
				{
					id: 'work',
					name: '  Work  ',
					launchIdentities: ['a', 'a', '', 42, 'b'],
					closeIdentities: ['c'],
					createdAt: 1700000000000,
				},
				// A duplicate id would make rename and delete ambiguous.
				{ id: 'work', name: 'Work again', launchIdentities: [] },
				{ id: '', name: 'No id' },
				{ id: 'unnamed', name: '   ' },
				'not an object',
			],
		})

		expect(normalized.scenarios).toEqual([
			{
				id: 'work',
				name: 'Work',
				launchIdentities: ['a', 'b'],
				closeIdentities: ['c'],
				createdAt: 1700000000000,
			},
		])
	})

	// v11 stored scenarios without a creation date. Stamping them during the migration would
	// claim they were made today, so they stay undated and the row simply omits it.
	it('upgrades a v11 scenario to an undated one and keeps the rest of it', () => {
		const normalized = normalizePreferences({
			version: 11,
			scenarios: [
				{
					id: 'work',
					name: 'Work',
					launchIdentities: ['a'],
					closeIdentities: ['b'],
				},
			],
		})

		expect(normalized.version).toBe(14)
		expect(normalized.scenarios).toEqual([
			{
				id: 'work',
				name: 'Work',
				launchIdentities: ['a'],
				closeIdentities: ['b'],
				createdAt: null,
			},
		])
	})

	// v12 had no starred scenarios, so the upgrade defaults them and leaves the scenarios alone.
	it('upgrades a v12 document to no starred scenarios', () => {
		const normalized = normalizePreferences({
			version: 12,
			scenarios: [{ id: 'work', name: 'Work', createdAt: 1700000000000 }],
		})

		expect(normalized.version).toBe(14)
		expect(normalized.favoriteScenarioIds).toEqual([])
		expect(normalized.scenarios).toHaveLength(1)
	})

	// A star that names nothing would render a row with no scenario behind it.
	it('keeps only starred scenarios the document still has', () => {
		const normalized = normalizePreferences({
		version: 14,
			scenarios: [{ id: 'work', name: 'Work' }],
			favoriteScenarioIds: ['work', 'work', 'deleted', '', 42],
		})

		expect(normalized.favoriteScenarioIds).toEqual(['work'])
	})

	it('keeps the newest fifty valid scenario run records', () => {
		const normalized = normalizePreferences({
			version: 15,
			scenarioHistory: Array.from({ length: 55 }, (_, index) => ({
				id: `run-${index}`,
				scenarioId: 'work',
				scenarioName: 'Work',
				startedAt: 1000 + index,
				finishedAt: 2000 + index,
				launched: 1,
				closed: 0,
				notRunning: 0,
				unavailable: 0,
				blocked: 0,
				failed: 0,
				cancelled: false,
			})),
		})

		expect(normalized.scenarioHistory).toHaveLength(50)
		expect(normalized.scenarioHistory[0]?.id).toBe('run-54')
	})

	it('rejects a malformed creation date instead of showing it', () => {
		const normalized = normalizePreferences({
			version: 12,
			scenarios: [
				{ id: 'a', name: 'Zero', createdAt: 0 },
				{ id: 'b', name: 'Text', createdAt: 'yesterday' },
				{ id: 'c', name: 'Infinite', createdAt: Number.POSITIVE_INFINITY },
			],
		})

		expect(
			normalized.scenarios.map(scenario => scenario.createdAt),
		).toEqual([null, null, null])
	})

	it('caps the scenario list and each scenario list', () => {
		const normalized = normalizePreferences({
			version: 12,
			scenarios: Array.from({ length: 80 }, (_, index) => ({
				id: `s${index}`,
				name: `Scenario ${index}`,
				launchIdentities: Array.from(
					{ length: 40 },
					(_, entry) => `app-${entry}`,
				),
				closeIdentities: [],
			})),
		})

		// One click runs the whole scenario, so the work it can start is bounded on read too —
		// a hand-edited document cannot smuggle in an unbounded batch.
		expect(normalized.scenarios).toHaveLength(50)
		expect(normalized.scenarios[0]?.launchIdentities).toHaveLength(20)
	})

	it('keeps only valid durable override identities', () => {
		expect(
			normalizePreferences({
				version: 7,
				categoryOverrideIdentities: {
					'ci:codex': 'ai',
					'ci:wow': 'games',
					'ci:broken': 'missing',
					'': 'games',
				},
			}).categoryOverrideIdentities,
		).toEqual({ 'ci:codex': 'ai', 'ci:wow': 'games' })
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
		version: 14,
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
			categoryOverrideIdentities: {},
			hiddenAppIds: [],
			hiddenAppIdentities: [],
			promotedAppIds: [],
			promotedAppIdentities: [],
			installerAppIds: [],
			installerAppIdentities: [],
			scenarios: [],
		favoriteScenarioIds: [],
		scenarioHistory: [],
			firstSeenAt: {},
			legacyCanonicalPreferences: {
				favorite: [],
				hidden: [],
				promoted: [],
				installer: [],
				categoryOverrides: {},
			},
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

	// A newer build may have written a version 14 document; this build (v13) must not overwrite it
	// with the older shape and strip the fields it does not know about.
	it('does not overwrite a document written by a newer version', () => {
		const future = JSON.stringify({
			version: 15,
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
