import { renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { useCatalogView } from '../../../src/hooks/useCatalogView'
import type { AppInfo } from '../../../src/types'

function app(id: string, artifactKind?: AppInfo['artifactKind']): AppInfo {
	return {
		id,
		name: id,
		path: `C:\\${id}.exe`,
		iconBase64: null,
		artifactKind,
		category: artifactKind ? 'installers_docs' : 'other',
		launchKind: 'executable',
		sourceKind: 'portable',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
	}
}

describe('useCatalogView', () => {
	it('keeps installers and docs out of the Ctrl+K palette', () => {
		const { result } = renderHook(() =>
			useCatalogView({
				activeView: 'all',
				apps: [
					app('Editor'),
					app('Editor Setup', 'installer'),
					app('Editor Help', 'documentation'),
				],
				categoryOverrideIdentities: {},
				categoryOverrides: {},
				favoriteAppIds: [],
				hiddenAppIds: [],
				promotedAppIdentities: [],
				promotedAppIds: [],
				query: '',
			}),
		)

		expect(result.current.paletteApps.map(item => item.id)).toEqual(['Editor'])
	})
})
