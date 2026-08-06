import { render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { FavoritesGrid } from '../../../../src/widgets/catalog-content/ui/FavoritesGrid'
import type { AppInfo } from '../../../../src/entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../src/entities/category'

vi.mock(
	'../../../../src/widgets/catalog-content/ui/CatalogAppCard/CatalogAppCard',
	() => ({
		CatalogAppCard: ({ app }: { app: AppInfo }) => (
			<button type='button' aria-label={`Launch ${app.name}`}>
				{app.name}
			</button>
		),
	}),
)

const games: CategoryDefinition = {
	id: 'games',
	label: 'Games',
	builtIn: true,
}

function favorite(id: string, name: string): AppInfo {
	return {
		id,
		name,
		path: `C:\\Games\\${id}.exe`,
		iconBase64: null,
		category: 'games',
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
	}
}

function props(apps: AppInfo[]) {
	return {
		apps,
		hasQuery: false,
		favoriteAppIds: apps.map(app => app.id),
		categories: [games],
		categoryOrder: ['games'] as AppCategory[],
		onToggleFavorite: vi.fn(),
		onLaunch: vi.fn().mockResolvedValue(undefined),
		onMoveApp: vi.fn(),
		onInfo: vi.fn(),
		onUninstall: vi.fn(),
		onHide: vi.fn(),
		onRestore: vi.fn(),
		onDemote: vi.fn(),
	}
}

describe('FavoritesGrid', () => {
	it('titles the view and counts what it shows', () => {
		render(
			<FavoritesGrid
				{...props([favorite('steam', 'Steam'), favorite('gog', 'GOG')])}
			/>,
		)

		// The heading is the view's only label once the filter tiles are gone, and the count has
		// to be the size of this list — the app header counts the whole catalog.
		const view = screen.getByRole('region', { name: 'Favorites' })
		expect(
			within(view).getByRole('heading', { level: 1, name: 'Favorites' }),
		).toBeInTheDocument()
		expect(view).toHaveTextContent('2 applications')
		expect(within(view).getAllByRole('button')).toHaveLength(2)
	})

	it('says application in the singular for one favorite', () => {
		render(<FavoritesGrid {...props([favorite('steam', 'Steam')])} />)

		expect(
			screen.getByRole('region', { name: 'Favorites' }),
		).toHaveTextContent('1 application')
	})

	it('keeps the empty state instead of a heading over nothing', () => {
		render(<FavoritesGrid {...props([])} />)

		expect(screen.getByText('No favorites yet')).toBeInTheDocument()
		expect(
			screen.queryByRole('heading', { level: 1, name: 'Favorites' }),
		).not.toBeInTheDocument()
	})
})
