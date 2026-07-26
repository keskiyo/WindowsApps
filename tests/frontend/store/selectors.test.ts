import { describe, expect, it } from 'vitest'
import {
	filterAppsByQuery,
	rankAppsByQuery,
	selectCategorizedApps,
} from '../../../src/store/selectors'
import type { AppCategory, AppInfo } from '../../../src/types'

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
) {
	return {
		apps,
		categoryOverrides,
		promotedAppIds,
		promotedAppIdentities: [] as string[],
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

	it('keeps identity when an override resolves to the current category', () => {
		const result = selectCategorizedApps(state(catalog, { steam: 'games' }))
		expect(result[0]).toBe(catalog[0])
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
