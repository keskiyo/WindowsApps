import { renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { useCatalogView } from '../../../../src/widgets/catalog-content/model/useCatalogView'
import type { AppInfo, AppView } from '../../../../src/entities/app'
import { DEFAULT_CATEGORIES } from '../../../../src/entities/category'
import type { Scenario } from '../../../../src/entities/scenario'

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

function scenario(value: Partial<Scenario> & Pick<Scenario, 'id'>): Scenario {
	return {
		name: value.id,
		launchIdentities: [],
		closeIdentities: [],
		createdAt: null,
		...value,
	}
}

function view(state: {
	activeView?: AppView
	apps?: AppInfo[]
	scenarios?: Scenario[]
}) {
	return renderHook(() =>
		useCatalogView({
			activeView: state.activeView ?? 'all',
			apps: state.apps ?? [],
			categories: DEFAULT_CATEGORIES,
			categoryOverrideIdentities: {},
			categoryOverrides: {},
			favoriteAppIds: [],
			favoriteAppIdentities: [],
			firstSeenAt: {},
			scenarios: state.scenarios ?? [],
			hiddenAppIds: [],
			hiddenAppIdentities: [],
			promotedAppIdentities: [],
			installerAppIds: [],
			installerAppIdentities: [],
			promotedAppIds: [],
			query: '',
		}),
	)
}

describe('useCatalogView', () => {
	it('keeps installers and docs out of the Ctrl+K palette', () => {
		const { result } = view({
			apps: [
				app('Editor'),
				app('Editor Setup', 'installer'),
				app('Editor Help', 'documentation'),
			],
		})

		expect(result.current.primaryApps.map(item => item.id)).toEqual([
			'Editor',
		])
	})

	describe('icon hydration', () => {
		// Scenario rows show each app by its icon, and that page renders no catalog grid — without
		// its own hydration the tiles would sit on the fallback glyph.
		it('covers the apps a scenario holds while the scenarios page is open', () => {
			const { result } = view({
				activeView: 'scenarios',
				apps: [app('Editor'), app('Chat'), app('Music')],
				scenarios: [
					scenario({
						id: 'work',
						launchIdentities: ['Editor'],
						closeIdentities: ['Chat'],
					}),
				],
			})

			expect(
				result.current.visibleHydrationIds.split('|').sort(),
			).toEqual(['Chat', 'Editor'])
		})

		it('asks for one app once when both lists hold it', () => {
			const { result } = view({
				activeView: 'scenarios',
				apps: [app('Editor')],
				scenarios: [
					scenario({
						id: 'work',
						launchIdentities: ['Editor'],
						closeIdentities: ['Editor'],
					}),
					scenario({ id: 'other', launchIdentities: ['Editor'] }),
				],
			})

			expect(result.current.visibleHydrationIds).toBe('Editor')
		})

		// More previews apps in its cards and reaches the scenario apps through the run dialog.
		it('covers what the More page shows, previews and scenarios alike', () => {
			const { result } = view({
				activeView: 'more',
				apps: [app('Editor'), app('Setup', 'installer')],
				scenarios: [
					scenario({ id: 'work', launchIdentities: ['Editor'] }),
				],
			})

			expect(
				result.current.visibleHydrationIds.split('|').sort(),
			).toEqual(['Editor', 'Setup'])
		})

		it('asks for nothing on a page that shows no apps', () => {
			const { result } = view({
				activeView: 'settings',
				apps: [app('Editor')],
				scenarios: [
					scenario({ id: 'work', launchIdentities: ['Editor'] }),
				],
			})

			expect(result.current.visibleHydrationIds).toBe('')
		})
	})

	describe('More preview', () => {
		// Three scenarios fit the card exactly, so hiding one behind a "View all" row that leads to
		// the same three is pure noise. The row only earns its slot once something is left out.
		it('previews every scenario while they all fit, newest first', () => {
			const { result } = view({
				scenarios: [
					scenario({ id: 'oldest', createdAt: 1_000 }),
					scenario({ id: 'newest', createdAt: 3_000 }),
					scenario({ id: 'middle', createdAt: 2_000 }),
				],
			})

			expect(
				result.current.morePreview.scenarios.map(entry => entry.id),
			).toEqual(['newest', 'middle', 'oldest'])
			expect(result.current.morePreview.auxiliary).toHaveLength(0)
		})

		// The fourth scenario costs the third slot, which the "View all" row then takes.
		it('gives up a slot to the view-all row once one scenario is left out', () => {
			const { result } = view({
				scenarios: [
					scenario({ id: 'oldest', createdAt: 1_000 }),
					scenario({ id: 'newest', createdAt: 4_000 }),
					scenario({ id: 'middle', createdAt: 2_000 }),
					scenario({ id: 'later', createdAt: 3_000 }),
				],
			})

			expect(
				result.current.morePreview.scenarios.map(entry => entry.id),
			).toEqual(['newest', 'later'])
		})

		it('previews three apps for the areas that have no extra row', () => {
			const { result } = view({
				activeView: 'auxiliary',
				apps: [
					app('Setup A', 'installer'),
					app('Setup B', 'installer'),
					app('Setup C', 'installer'),
					app('Setup D', 'installer'),
				],
			})

			expect(result.current.morePreview.installersDocs).toHaveLength(3)
		})

		// A scenario stored before creation dates existed cannot claim to be the newest one.
		it('puts scenarios with no creation date last', () => {
			const { result } = view({
				scenarios: [
					scenario({ id: 'undated' }),
					scenario({ id: 'dated', createdAt: 1_000 }),
				],
			})

			expect(
				result.current.morePreview.scenarios.map(entry => entry.id),
			).toEqual(['dated', 'undated'])
		})
	})
})
