import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { FavoritesGrid } from '../../../../src/widgets/catalog-content/ui/FavoritesGrid'
import type { AppInfo } from '../../../../src/entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../src/entities/category'
import type { Scenario } from '../../../../src/entities/scenario'

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

const gaming: Scenario = {
	id: 'gaming',
	name: 'Gaming',
	launchIdentities: ['steam'],
	closeIdentities: [],
	createdAt: null,
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

function props(apps: AppInfo[], scenarios: Scenario[] = []) {
	return {
		apps,
		hasQuery: false,
		favoriteAppIds: apps.map(app => app.id),
		categories: [games],
		categoryOrder: ['games'] as AppCategory[],
		favoriteScenarios: {
			scenarios,
			apps,
			runningId: null,
			onRun: vi.fn(),
			onToggleFavorite: vi.fn(),
		},
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

	it('names both blocks of the view and what each one holds', () => {
		render(<FavoritesGrid {...props([favorite('steam', 'Steam')], [gaming])} />)

		const scenarios = screen.getByRole('region', { name: 'Scenarios' })
		expect(scenarios).toHaveTextContent('1 scenario')
		expect(scenarios).toHaveTextContent('Run your configured scenarios')
		expect(
			screen.getByRole('heading', { level: 2, name: 'Applications' }),
		).toBeInTheDocument()
		expect(
			screen.getByText('Your installed applications'),
		).toBeInTheDocument()
		expect(
			screen.getByRole('region', { name: 'Favorites' }),
		).toHaveTextContent('1 application')
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

	it('lists favorite scenarios collapsed and expands one on its name', async () => {
		const steam = favorite('steam', 'Steam')
		render(<FavoritesGrid {...props([steam], [gaming])} />)

		const section = screen.getByRole('region', { name: 'Scenarios' })
		expect(section).toHaveTextContent('1 scenario')
		expect(
			screen.queryByRole('list', { name: 'Launch list of Gaming' }),
		).not.toBeInTheDocument()

		await userEvent.click(within(section).getByText('Gaming'))

		expect(
			screen.getByRole('list', { name: 'Launch list of Gaming' }),
		).toHaveTextContent('Steam')
	})

	it('shows the scenarios section even when no app is starred', () => {
		render(<FavoritesGrid {...props([], [gaming])} />)

		expect(screen.getByRole('region', { name: 'Scenarios' })).toBeInTheDocument()
		expect(screen.queryByText('No favorites yet')).not.toBeInTheDocument()
	})

	it('unstars a scenario from its own card', async () => {
		const view = props([], [gaming])
		render(<FavoritesGrid {...view} />)

		await userEvent.click(
			screen.getByRole('button', { name: 'Remove Gaming from favorites' }),
		)

		expect(view.favoriteScenarios.onToggleFavorite).toHaveBeenCalledWith('gaming')
	})
})
