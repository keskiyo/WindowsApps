import { describe, expect, it } from 'vitest'
import { deduplicateVisibleApps } from '../../lib/appDeduplication'
import type { AppInfo } from '../../types'

function app(
	value: Partial<AppInfo> & Pick<AppInfo, 'id' | 'name' | 'path'>,
): AppInfo {
	return {
		category: 'games',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
		...value,
	}
}

describe('deduplicateVisibleApps', () => {
	it('keeps distinct apps untouched', () => {
		const apps = [
			app({ id: 'steam', name: 'Steam', path: 'C:\\Steam\\steam.exe' }),
			app({
				id: 'chrome',
				name: 'Chrome',
				path: 'C:\\Chrome\\chrome.exe',
			}),
		]
		expect(deduplicateVisibleApps(apps)).toHaveLength(2)
	})

	it('collapses the same app found at the same path and merges metadata', () => {
		const apps = [
			app({
				id: 'a',
				name: 'Discord',
				path: 'C:\\Discord\\Discord.exe',
				iconBase64: null,
			}),
			app({
				id: 'b',
				name: 'Discord',
				path: 'C:\\Discord\\Discord.exe',
				launchKind: 'shortcut',
				iconBase64: 'data:image/png;base64,xxx',
				canUninstall: true,
			}),
		]
		const result = deduplicateVisibleApps(apps)
		expect(result).toHaveLength(1)
		// Shortcut scores higher, so it survives and absorbs the missing icon stays available.
		expect(result[0].iconBase64).toBe('data:image/png;base64,xxx')
		expect(result[0].canUninstall).toBe(true)
	})

	it('does not merge apps from conflicting publishers', () => {
		const apps = [
			app({
				id: 'a',
				name: 'Setup',
				path: 'C:\\One\\setup.exe',
				publisher: 'Acme Inc',
			}),
			app({
				id: 'b',
				name: 'Setup',
				path: 'C:\\Two\\setup.exe',
				publisher: 'Globex LLC',
			}),
		]
		expect(deduplicateVisibleApps(apps)).toHaveLength(2)
	})
})
