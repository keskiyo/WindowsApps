import { describe, expect, it } from 'vitest'
import {
	filterAppsByQuery,
	filterVisibleApps,
	rankAppsByQuery,
	selectCatalogCounts,
	selectCategorizedApps,
} from '../../../../src/app/store/selectors'
import { selectRecentApps } from '../../../../src/entities/app'
import type { AppInfo } from '../../../../src/entities/app'
import type { AppCategory } from '../../../../src/entities/category'

function app(
	value: Partial<AppInfo> & Pick<AppInfo, 'id' | 'name'>,
): AppInfo {
	return {
		path: `C:\\Program Files\\${value.id}\\app.exe`,
		category: 'other',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
		...value,
	} as AppInfo
}

function state(
	apps: AppInfo[],
	categoryOverrides: Record<string, AppCategory> = {},
	promotedAppIds: string[] = [],
	categoryOverrideIdentities: Record<string, AppCategory> = {},
	installerAppIds: string[] = [],
	installerAppIdentities: string[] = [],
) {
	return {
		apps,
		categoryOverrides,
		categoryOverrideIdentities,
		promotedAppIds,
		promotedAppIdentities: [] as string[],
		installerAppIds,
		installerAppIdentities,
	}
}

const catalog = [
	app({ id: 'steam', name: 'Steam', category: 'games' }),
	app({ id: 'code', name: 'Visual Studio Code', category: 'development' }),
	app({ id: 'notepad', name: 'Notepad', category: 'utilities' }),
]

describe('categorized app identity', () => {
	it('returns the original records when nothing applies to them', () => {
		const result = selectCategorizedApps(state(catalog))
		expect(result).toHaveLength(catalog.length)
		result.forEach((entry, index) => {
			expect(entry).toBe(catalog[index])
		})
	})

	it('creates a new record only for the app whose category was overridden', () => {
		const result = selectCategorizedApps(
			state(catalog, { code: 'utilities' }),
		)
		expect(result[0]).toBe(catalog[0])
		expect(result[2]).toBe(catalog[2])
		expect(result[1]).not.toBe(catalog[1])
		expect(result[1].category).toBe('utilities')
	})

	it('applies a durable override by canonical identity even when the app id changed', () => {
		// Simulates a Force full scan / dedup change: a new id, the same canonical identity, and only
		// the identity-keyed map carries the override. The manual category must still follow the app.
		const rescanned = [
			app({
				id: 'code-v2',
				name: 'Visual Studio Code',
				category: 'development',
				canonicalIdentity: 'ci:vscode',
			}),
		]
		const result = selectCategorizedApps(
			state(rescanned, {}, [], { 'ci:vscode': 'utilities' }),
		)
		expect(result[0].category).toBe('utilities')
	})

	it('keeps identity when an override resolves to the current category', () => {
		const result = selectCategorizedApps(state(catalog, { steam: 'games' }))
		expect(result[0]).toBe(catalog[0])
	})

	it('turns a manual mark into an installer artifact', () => {
		const result = selectCategorizedApps(
			state(catalog, {}, [], {}, ['notepad']),
		)

		// The Installers & Docs view selects on `artifactKind`, so a category alone would leave the
		// app invisible in the bucket the user just moved it to.
		expect(result[2].artifactKind).toBe('installer')
		expect(result[2].category).toBe('installers_docs')
		expect(result[2].userInstaller).toBe(true)
		expect(filterVisibleApps(result, 'installers_docs', [], [])).toEqual([
			result[2],
		])
		expect(filterVisibleApps(result, 'all', [], [])).not.toContain(result[2])
	})

	it('follows a manual installer mark by canonical identity after a rescan', () => {
		const rescanned = [
			app({
				id: 'notepad-v2',
				name: 'Notepad',
				category: 'utilities',
				canonicalIdentity: 'ci:notepad',
			}),
		]

		const result = selectCategorizedApps(
			state(rescanned, {}, [], {}, [], ['ci:notepad']),
		)

		expect(result[0].artifactKind).toBe('installer')
	})

	it('previews the highest-ranked entries of an area', () => {
		const rank = new Map([
			['steam', 30],
			['code', 10],
			['notepad', 20],
		])

		expect(
			selectRecentApps(catalog, app => rank.get(app.id) ?? 0, 2).map(
				app => app.id,
			),
		).toEqual(['steam', 'notepad'])
	})

	it('falls back to alphabetical order when nothing is newer', () => {
		// A first run stamps the whole catalog at once, so every rank ties; scan order would make
		// the preview look arbitrary and reshuffle on the next scan.
		expect(
			selectRecentApps(catalog, () => 0, 3).map(app => app.name),
		).toEqual(['Notepad', 'Steam', 'Visual Studio Code'])
	})

	it('leaves the source array alone', () => {
		const source = [...catalog]
		selectRecentApps(source, app => (app.id === 'notepad' ? 1 : 0), 3)
		expect(source).toEqual(catalog)
	})

	it('leaves untouched records stable when a hydration patch arrives', () => {
		const before = selectCategorizedApps(state(catalog))
		// applyPatches replaces only the patched record and keeps the other references.
		const patched = catalog.map(entry =>
			entry.id === 'code' ? { ...entry, iconBase64: 'data:png' } : entry,
		)
		const after = selectCategorizedApps(state(patched))
		expect(after[0]).toBe(before[0])
		expect(after[2]).toBe(before[2])
		expect(after[1]).not.toBe(before[1])
		expect(after[1].iconBase64).toBe('data:png')
	})

	it('promotes an auxiliary app the user selected', () => {
		const auxiliary = [
			app({
				id: 'helper',
				name: 'Helper',
				category: 'system',
				visibilityClass: 'auxiliary',
			}),
		]
		const result = selectCategorizedApps(
			state(auxiliary, {}, ['helper']),
		)
		expect(result[0]).not.toBe(auxiliary[0])
		expect(result[0].visibilityClass).toBe('primary')
		expect(result[0].userPromoted).toBe(true)
	})
})

describe('selectCatalogCounts', () => {
	// steam: favorite + visible. code: favorite but hidden. notepad: plain visible.
	// helper: auxiliary. legacy: auxiliary and hidden.
	const mixed = [
		app({ id: 'steam', name: 'Steam', category: 'games' }),
		app({ id: 'code', name: 'Code', category: 'development' }),
		app({ id: 'notepad', name: 'Notepad', category: 'utilities' }),
		app({
			id: 'helper',
			name: 'Helper',
			category: 'system',
			visibilityClass: 'auxiliary',
		}),
		app({
			id: 'legacy',
			name: 'Legacy',
			category: 'system',
			visibilityClass: 'auxiliary',
		}),
	]
	const hiddenAppIds = ['code', 'legacy']
	const favoriteAppIds = ['steam', 'code']

	// The contract that keeps a badge from contradicting the list it opens. Each count is the
	// length of `filterVisibleApps` for the matching view — checked against that function rather
	// than a hand-written number, so the two cannot drift apart.
	it.each(['favorites', 'hidden', 'auxiliary'] as const)(
		'reports the %s badge as the length of that view',
		view => {
			const counts = selectCatalogCounts(
				mixed,
				hiddenAppIds,
				favoriteAppIds,
			)
			const badge = {
				favorites: counts.favoriteCount,
				hidden: counts.hiddenCount,
				auxiliary: counts.auxiliaryCount,
			}[view]

			expect(badge).toBe(
				filterVisibleApps(mixed, view, hiddenAppIds, favoriteAppIds)
					.length,
			)
		},
	)

	it('excludes auxiliary and hidden apps from the visible set and its category counts', () => {
		const counts = selectCatalogCounts(mixed, hiddenAppIds, favoriteAppIds)

		expect(counts.visibleCategorizedApps.map(entry => entry.id)).toEqual([
			'steam',
			'notepad',
		])
		expect(counts.navigationCounts.get('games')).toBe(1)
		expect(counts.navigationCounts.get('development')).toBeUndefined()
		expect(counts.navigationCounts.get('system')).toBeUndefined()
	})

	// The settings page reports what the scanner classified, so hiding an app must not change it.
	it('keeps classification totals independent of what the user hid', () => {
		const counts = selectCatalogCounts(mixed, hiddenAppIds, favoriteAppIds)

		expect(counts.classifiedAuxiliaryCount).toBe(2)
		expect(counts.classifiedPrimaryCount).toBe(3)
		expect(
			counts.classifiedPrimaryCount + counts.classifiedAuxiliaryCount,
		).toBe(mixed.length)
	})

	it('preserves record identity in the visible set', () => {
		const counts = selectCatalogCounts(mixed, [], [])

		expect(counts.visibleCategorizedApps[0]).toBe(mixed[0])
	})

	it('isolates installers and docs from ordinary views and counts them in the reserved category', () => {
		const installer = app({
			id: 'setup',
			name: 'Editor Setup',
			artifactKind: 'installer',
			category: 'installers_docs',
		})
		const docs = app({
			id: 'docs',
			name: 'Editor Help',
			artifactKind: 'documentation',
			category: 'installers_docs',
		})
		const apps = [catalog[0]!, installer, docs]

		expect(filterVisibleApps(apps, 'all', [], ['setup'])).toEqual([
			catalog[0],
		])
		expect(filterVisibleApps(apps, 'favorites', [], ['setup'])).toEqual([])
		expect(filterVisibleApps(apps, 'auxiliary', [], ['setup'])).toEqual([])
		expect(filterVisibleApps(apps, 'installers_docs', [], [])).toEqual([
			installer,
			docs,
		])
		const counts = selectCatalogCounts(apps, [], ['setup'])
		expect(counts.visibleCategorizedApps).toEqual([catalog[0]])
		expect(counts.navigationCounts.get('installers_docs')).toBe(2)
		expect(counts.favoriteCount).toBe(0)
	})
})

describe('search index freshness', () => {
	it('finds a record by metadata that arrived after an earlier search', () => {
		const before = app({ id: 'editor', name: 'Editor' })
		expect(filterAppsByQuery([before], 'contoso')).toHaveLength(0)

		// Hydration ships a new object for the same id, so the cached haystack of the
		// previous record must not hide the newly arrived publisher.
		const hydrated = { ...before, publisher: 'Contoso Ltd' }
		expect(filterAppsByQuery([hydrated], 'contoso')).toEqual([hydrated])
	})

	it('matches tokens across different fields', () => {
		const target = app({
			id: 'wow',
			name: 'World of Warcraft',
			publisher: 'Blizzard',
		})
		expect(filterAppsByQuery([target], 'warcraft blizzard')).toEqual([
			target,
		])
		expect(filterAppsByQuery([target], 'warcraft missing')).toHaveLength(0)
	})
})

describe('rankAppsByQuery', () => {
	it('ranks a name match above a match that only hits the install path', () => {
		const telegram = app({
			id: 'tg',
			name: 'Telegram',
			path: 'C:\\Apps\\Telegram\\telegram.exe',
		})
		const byPath = app({
			id: 'tool',
			name: 'Some Tool',
			path: 'C:\\telemetry\\tool.exe',
		})

		const ranked = rankAppsByQuery([byPath, telegram], 'tel')
		expect(ranked.map(entry => entry.id)).toEqual(['tg', 'tool'])
	})

	it('does not let a short token match only through the path', () => {
		const byPathOnly = app({
			id: 'ps',
			name: 'Photoshop',
			path: 'C:\\ms\\ps.exe',
		})
		// "ms" appears only in the path and is shorter than the secondary-field threshold.
		expect(rankAppsByQuery([byPathOnly], 'ms')).toHaveLength(0)
	})

	it('prefers an exact/prefix name over a mid-word substring', () => {
		const code = app({ id: 'code', name: 'Code' })
		const xcode = app({ id: 'xcode', name: 'XCode Companion' })
		const ranked = rankAppsByQuery([xcode, code], 'code')
		expect(ranked[0].id).toBe('code')
	})

	it('requires every token to match (AND) and returns the input for an empty query', () => {
		const wow = app({ id: 'wow', name: 'World of Warcraft' })
		expect(rankAppsByQuery([wow], 'world warcraft')).toHaveLength(1)
		expect(rankAppsByQuery([wow], 'world nonsense')).toHaveLength(0)
		const list = [wow]
		expect(rankAppsByQuery(list, '   ')).toBe(list)
	})
})
