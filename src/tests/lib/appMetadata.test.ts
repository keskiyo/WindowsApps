import { describe, expect, it } from 'vitest'
import { descriptionLabel, metadataRows } from '../../lib/appMetadata'

describe('application metadata formatting', () => {
	it('does not invent an unavailable description', () => {
		expect(descriptionLabel(null)).toBe('No description available')
	})

	it('uses explicit unknown fallbacks for absent fields', () => {
		expect(
			metadataRows(
				{
					version: null,
					publisher: null,
					category: 'other',
					sourceKind: 'start_menu',
					path: 'C:\\App.lnk',
					installLocation: null,
				},
				[{ id: 'other', label: 'Other', builtIn: true }],
			),
		).toContainEqual(['Version', 'Unknown'])
	})

	it('explains why an auxiliary tool was separated from the catalog', () => {
		const rows = metadataRows(
			{
				version: '1.0',
				publisher: null,
				category: 'development',
				sourceKind: 'portable',
				path: String.raw`C:\Git\usr\bin\iconv.exe`,
				installLocation: String.raw`C:\Git`,
				visibilityClass: 'auxiliary',
				visibilityScore: -30,
				visibilityReasons: ['runtime_directory', 'product_component'],
			},
			[{ id: 'development', label: 'Development', builtIn: true }],
		)

		expect(rows).toContainEqual(['Catalog visibility', 'Auxiliary tool'])
		expect(rows).toContainEqual([
			'Classification reasons',
			'Runtime directory, Product component',
		])
	})

	it('shows classification score only when diagnostics are requested', () => {
		const app = {
			version: null,
			publisher: null,
			category: 'other' as const,
			sourceKind: 'portable' as const,
			path: 'C:\\Tool.exe',
			installLocation: null,
			visibilityScore: -20,
		}
		const categories = [{ id: 'other' as const, label: 'Other', builtIn: true }]
		expect(metadataRows(app, categories)).not.toContainEqual([
			'Classification score',
			'-20',
		])
		expect(metadataRows(app, categories, true)).toContainEqual([
			'Classification score',
			'-20',
		])
	})
})
