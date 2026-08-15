import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { HiddenGrid } from '../../../../src/widgets/catalog-content/ui/HiddenGrid'
import type { AppInfo } from '../../../../src/entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../src/entities/category'

vi.mock(
	'../../../../src/widgets/catalog-content/ui/CatalogAppCard/CatalogAppCard',
	() => ({
		CatalogAppCard: ({ app }: { app: AppInfo }) => (
			<article>{app.name}</article>
		),
	}),
)

const utilities: CategoryDefinition = {
	id: 'utilities',
	label: 'Utilities',
	builtIn: true,
}

function hidden(id: string, name: string): AppInfo {
	return {
		id,
		name,
		path: `C:\\Tools\\${id}.exe`,
		iconBase64: null,
		category: 'utilities',
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
	}
}

function props(apps: AppInfo[], onBack = vi.fn()) {
	return {
		apps,
		hasQuery: false,
		favoriteAppIds: [],
		categories: [utilities],
		categoryOrder: ['utilities'] as AppCategory[],
		onBack,
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

describe('HiddenGrid', () => {
	it('titles the view and counts what it shows', () => {
		render(
			<HiddenGrid
				{...props([hidden('a', 'Alpha'), hidden('z', 'Zeta')])}
			/>,
		)

		const view = screen.getByRole('region', { name: 'Hidden' })
		expect(
			within(view).getByRole('heading', { level: 1, name: 'Hidden' }),
		).toBeInTheDocument()
		expect(view).toHaveTextContent('2 applications')
	})

	it('returns to More from the title row', async () => {
		const onBack = vi.fn()
		render(<HiddenGrid {...props([hidden('a', 'Alpha')], onBack)} />)

		await userEvent.click(
			screen.getByRole('button', { name: 'Back to More' }),
		)
		expect(onBack).toHaveBeenCalled()
	})

	it('keeps the title and the way back while nothing is hidden', async () => {
		const onBack = vi.fn()
		render(<HiddenGrid {...props([], onBack)} />)

		expect(screen.getByText('No hidden apps')).toBeVisible()
		expect(
			screen.getByRole('region', { name: 'Hidden' }),
		).toHaveTextContent('0 applications')
		await userEvent.click(
			screen.getByRole('button', { name: 'Back to More' }),
		)
		expect(onBack).toHaveBeenCalled()
	})
})
