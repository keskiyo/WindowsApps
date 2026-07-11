import { describe, expect, it } from 'vitest'
import { filterAppsByQuery, filterVisibleApps } from '../../store/selectors'
import type { AppInfo } from '../../types'

function app(index: number): AppInfo {
	const category = index % 3 === 0 ? 'development' : index % 3 === 1 ? 'games' : 'utilities'
	return {
		id: `app-${index}`,
		name: `Sample App ${index}`,
		path: `C:\\Program Files\\Sample ${index}\\Sample.exe`,
		category,
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: index % 100 === 0 ? 'Large catalog marker' : null,
		version: null,
		publisher: index % 2 === 0 ? 'Sample Publisher' : null,
		installLocation: `C:\\Program Files\\Sample ${index}`,
		canUninstall: false,
	}
}

describe('large catalog selector behavior', () => {
	it('filters a 10000 item catalog without changing result semantics', () => {
		const apps = Array.from({ length: 10000 }, (_, index) => app(index))
		const visible = filterVisibleApps(
			apps,
			'all',
			['app-1', 'app-9999'],
			['app-100', 'app-200'],
		)
		expect(visible).toHaveLength(9998)

		const searchMatches = filterAppsByQuery(visible, 'large marker')
		expect(searchMatches.map(item => item.id).slice(0, 3)).toEqual([
			'app-0',
			'app-100',
			'app-200',
		])
		expect(searchMatches).toHaveLength(100)
	})
})
