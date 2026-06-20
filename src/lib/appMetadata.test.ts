import { describe, expect, it } from 'vitest'
import { descriptionLabel, metadataRows } from './appMetadata'

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
})
