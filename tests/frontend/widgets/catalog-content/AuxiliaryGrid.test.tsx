import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AuxiliaryGrid } from '../../../../src/widgets/catalog-content/ui/AuxiliaryGrid'
import type { AppInfo } from '../../../../src/entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../src/entities/category'

vi.mock(
	'../../../../src/widgets/catalog-content/ui/AuxiliaryToolRow/AuxiliaryToolRow',
	() => ({
		AuxiliaryToolRow: ({ app }: { app: AppInfo }) => (
			<button type='button' aria-label={`Launch ${app.name}`}>
				{app.name}
			</button>
		),
	}),
)

const development: CategoryDefinition = {
	id: 'development',
	label: 'Development',
	builtIn: true,
}

function tool(id: string, name: string, publisher: string): AppInfo {
	return {
		id,
		name,
		path: `C:\\Tools\\${id}.exe`,
		iconBase64: null,
		category: 'development',
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher,
		installLocation: null,
		canUninstall: false,
	}
}

function props() {
	return {
		apps: [
			tool('zeta', 'Zeta', 'Anthropic PBC'),
			tool('alpha', 'Alpha', 'Devsense'),
		],
		hasQuery: false,
		favoriteAppIds: [],
		categories: [development],
		categoryOrder: ['development'] as AppCategory[],
		onBack: vi.fn(),
		onLaunch: vi.fn().mockResolvedValue(undefined),
		onMoveApp: vi.fn(),
		onInfo: vi.fn(),
		onUninstall: vi.fn(),
		onPromote: vi.fn(),
		onDemote: vi.fn(),
	}
}

describe('AuxiliaryGrid', () => {
	it('renders tools in one ungrouped alphabetical sequence', () => {
		render(<AuxiliaryGrid {...props()} />)

		expect(
			screen
				.getAllByRole('button', { name: /^Launch / })
				.map(button => button.getAttribute('aria-label')),
		).toEqual(['Launch Alpha', 'Launch Zeta'])
		expect(
			screen.queryByRole('region', { name: 'Anthropic PBC' }),
		).not.toBeInTheDocument()
		expect(
			screen.queryByRole('region', { name: 'Devsense' }),
		).not.toBeInTheDocument()
	})

	it('titles the view, counts its tools and returns to More', async () => {
		const onBack = vi.fn()
		render(<AuxiliaryGrid {...props()} onBack={onBack} />)

		const view = screen.getByRole('region', { name: 'Auxiliary tools' })
		expect(view).toHaveTextContent('2 tools')
		await userEvent.click(screen.getByRole('button', { name: 'Back to More' }))
		expect(onBack).toHaveBeenCalled()
	})

	it('keeps the way back when no tool matches the search', async () => {
		const onBack = vi.fn()
		render(
			<AuxiliaryGrid
				{...props()}
				apps={[]}
				hasQuery
				onBack={onBack}
			/>,
		)

		expect(screen.getByText('No matching auxiliary tools')).toBeVisible()
		await userEvent.click(screen.getByRole('button', { name: 'Back to More' }))
		expect(onBack).toHaveBeenCalled()
	})
})
