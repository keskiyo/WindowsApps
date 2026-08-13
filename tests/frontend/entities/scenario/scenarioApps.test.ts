import { describe, expect, it } from 'vitest'
import { resolveScenarioApps } from '../../../../src/entities/scenario'
import type { AppInfo } from '../../../../src/entities/app'

function app(value: Partial<AppInfo> & Pick<AppInfo, 'id'>): AppInfo {
	return {
		name: value.id,
		path: `C:\\Apps\\${value.id}.exe`,
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
	}
}

describe('resolveScenarioApps', () => {
	it('keeps the stored order rather than the catalog order', () => {
		const catalog = [app({ id: 'a' }), app({ id: 'b' }), app({ id: 'c' })]

		const resolved = resolveScenarioApps(['c', 'a'], catalog)

		expect(resolved.apps.map(entry => entry.id)).toEqual(['c', 'a'])
		expect(resolved.unavailable).toEqual([])
	})

	it('returns last known metadata for entries the catalog no longer contains', () => {
		const resolved = resolveScenarioApps(
			['gone', 'a', 'also-gone'],
			[app({ id: 'a' })],
			{
				gone: {
					name: 'ChatGPT',
					iconBase64: 'data:image/png;base64,AAAA',
				},
			},
		)

		expect(resolved.apps.map(entry => entry.id)).toEqual(['a'])
		expect(resolved.unavailable).toEqual([
			{
				identity: 'gone',
				name: 'ChatGPT',
				iconBase64: 'data:image/png;base64,AAAA',
			},
			{
				identity: 'also-gone',
				name: 'Unavailable application',
				iconBase64: null,
			},
		])
	})

	// The durable key is the preference identity, so a rescan that renames the catalog id must
	// still resolve — that is the whole reason scenarios do not store ids.
	it('resolves through the preference identity, not the catalog id', () => {
		const rescanned = [
			app({ id: 'code-v2', preferenceIdentity: 'preference:code' }),
		]

		const resolved = resolveScenarioApps(['preference:code'], rescanned)

		expect(resolved.apps.map(entry => entry.id)).toEqual(['code-v2'])
		expect(resolved.unavailable).toEqual([])
	})
})
