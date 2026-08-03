import { describe, expect, it } from 'vitest'
import {
	rankAppsByQuery,
	rankAppsByQueryTop,
} from '../../../src/entities/app/lib/catalogSearch'
import type { AppInfo } from '../../../src/entities/app'

const PALETTE_LIMIT = 50

function app(index: number, name: string, publisher: string | null): AppInfo {
	return {
		id: `app-${index}`,
		name,
		path: `C:\\Program Files\\Sample ${index}\\Sample.exe`,
		category: 'utilities',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher,
		installLocation: `C:\\Program Files\\Sample ${index}`,
		canUninstall: false,
	}
}

// Names are built so the fixture exercises every tier of the comparator: exact match, prefix
// match, word-prefix match, substring, publisher-only, and — crucially — long runs of entries
// whose score *and* name length tie, where only insertion stability keeps the order right.
function largeCatalog(size: number): AppInfo[] {
	return Array.from({ length: size }, (_, index) => {
		const remainder = index % 7
		const name =
			remainder === 0
				? 'App'
				: remainder === 1
					? `App ${index}`
					: remainder === 2
						? `Sample App ${index}`
						: remainder === 3
							? `MyApp${index}`
							: remainder === 4
								? `Tool ${index}`
								: remainder === 5
									? 'App'
									: `Zebra ${index}`
		return app(index, name, remainder === 4 ? 'App Publisher' : null)
	})
}

describe('bounded command-palette ranking', () => {
	const apps = largeCatalog(10000)

	// The palette shows at most 50 rows but scored, filtered and fully sorted every match first.
	// The bounded selection has to be an optimisation only: same items, same order, same ties.
	it.each(['app', 'a', 'tool', 'sample app', 'zebra'])(
		'returns exactly the first %o results of a full sort',
		query => {
			const expected = rankAppsByQuery(apps, query).slice(0, PALETTE_LIMIT)

			const actual = rankAppsByQueryTop(apps, query, PALETTE_LIMIT)

			expect(actual.map(item => item.id)).toEqual(
				expected.map(item => item.id),
			)
		},
	)

	it('matches the full sort when fewer results exist than the limit', () => {
		const expected = rankAppsByQuery(apps, 'zebra 9999').slice(
			0,
			PALETTE_LIMIT,
		)

		const actual = rankAppsByQueryTop(apps, 'zebra 9999', PALETTE_LIMIT)

		expect(actual.map(item => item.id)).toEqual(expected.map(item => item.id))
		expect(actual.length).toBeLessThan(PALETTE_LIMIT)
	})

	it('never returns more than the limit', () => {
		expect(rankAppsByQueryTop(apps, 'app', PALETTE_LIMIT)).toHaveLength(
			PALETTE_LIMIT,
		)
	})

	// An empty query is the palette's opening state; it must behave like the unbounded ranking.
	it('bounds an empty query to the limit without reordering', () => {
		const actual = rankAppsByQueryTop(apps, '   ', PALETTE_LIMIT)

		expect(actual).toEqual(apps.slice(0, PALETTE_LIMIT))
	})

	it('returns nothing for a non-positive limit', () => {
		expect(rankAppsByQueryTop(apps, 'app', 0)).toEqual([])
	})

	// The existing ranking rules are behaviour, not implementation: a literal match outranks a
	// keyboard-layout or typo correction, and the `cmd` alias stays scoped to Windows Terminal.
	it('preserves literal, layout and typo ranking through the bounded path', () => {
		const docker = app(1, 'Docker Desktop', null)
		const catalog = [app(2, 'Documents Helper', null), docker]

		expect(
			rankAppsByQueryTop(catalog, 'вщсйук', PALETTE_LIMIT)[0]?.name,
		).toBe('Docker Desktop')
		expect(rankAppsByQueryTop(catalog, 'doker', PALETTE_LIMIT)[0]?.name).toBe(
			'Docker Desktop',
		)
	})
})
