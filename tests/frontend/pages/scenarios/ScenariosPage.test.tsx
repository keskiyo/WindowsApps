import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeAll, describe, expect, it, vi } from 'vitest'
import { ScenariosPage } from '../../../../src/pages/scenarios'
import type { Scenario } from '../../../../src/entities/scenario'
import type { AppInfo } from '../../../../src/entities/app'

function app(id: string, name: string): AppInfo {
	return {
		id,
		name,
		path: `C:\\Apps\\${id}.exe`,
		category: 'other',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
	}
}

const catalog = [app('game', 'Backpack Battles'), app('chat', 'Discord')]

function props(scenarios: Scenario[] = []) {
	return {
		scenarios,
		apps: catalog,
		runningId: null,
		onBack: vi.fn(),
		onCreate: vi.fn().mockReturnValue({ ok: true, id: 'new' }),
		onRename: vi.fn().mockReturnValue({ ok: true }),
		onDelete: vi.fn(),
		onAddApp: vi.fn().mockReturnValue({ ok: true }),
		onRemoveApp: vi.fn(),
		onRun: vi.fn(),
	}
}

const gaming: Scenario = {
	id: 'gaming',
	name: 'Gaming',
	launchIdentities: ['game'],
	closeIdentities: [],
	createdAt: null,
}

// The picker keeps its highlighted row in view; jsdom has no scroller to do it with.
beforeAll(() => {
	Object.defineProperty(Element.prototype, 'scrollIntoView', {
		configurable: true,
		value: vi.fn(),
	})
})

describe('ScenariosPage', () => {
	it('titles the view, counts scenarios and returns to More', async () => {
		const view = props([gaming])
		render(<ScenariosPage {...view} />)

		const page = screen.getByRole('region', { name: 'Scenarios' })
		expect(page).toHaveTextContent('1 scenario')
		await userEvent.click(screen.getByRole('button', { name: 'Back to More' }))
		expect(view.onBack).toHaveBeenCalled()
	})

	it('creates a scenario from the inline field', async () => {
		const view = props()
		render(<ScenariosPage {...view} />)

		await userEvent.click(screen.getByRole('button', { name: 'Add scenario' }))
		await userEvent.type(
			screen.getByRole('textbox', { name: 'New scenario name' }),
			'Focus',
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Save scenario name' }),
		)

		expect(view.onCreate).toHaveBeenCalledWith('Focus')
	})

	it('renames a scenario in place', async () => {
		const view = props([gaming])
		render(<ScenariosPage {...view} />)

		await userEvent.click(screen.getByRole('button', { name: 'Rename Gaming' }))
		const field = screen.getByRole('textbox', { name: 'Rename Gaming' })
		await userEvent.clear(field)
		await userEvent.type(field, 'Evening{Enter}')

		expect(view.onRename).toHaveBeenCalledWith('gaming', 'Evening')
	})

	it('picks an app for a list through the modal and adds it', async () => {
		const view = props([gaming])
		render(<ScenariosPage {...view} />)

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Add an app to the Close list of Gaming',
			}),
		)
		const picker = screen.getByRole('dialog', {
			name: 'Add an app to the close list of Gaming',
		})
		await userEvent.click(within(picker).getByRole('button', { name: /Discord/ }))

		expect(view.onAddApp).toHaveBeenCalledWith('gaming', 'close', 'chat')
		expect(
			screen.queryByRole('dialog', { name: /Add an app/ }),
		).not.toBeInTheDocument()
	})

	it('runs a scenario and blocks a second run while it is going', async () => {
		const view = { ...props([gaming]), runningId: 'gaming' }
		render(<ScenariosPage {...view} />)

		const run = screen.getByRole('button', { name: 'Run Gaming' })
		expect(run).toBeDisabled()
		expect(run).toHaveTextContent('Running…')
	})

	it('removes an app from a list', async () => {
		const view = props([gaming])
		render(<ScenariosPage {...view} />)

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Remove Backpack Battles from the Launch list of Gaming',
			}),
		)

		expect(view.onRemoveApp).toHaveBeenCalledWith('gaming', 'launch', 'game')
	})

	// An entry that stopped resolving must be visible, not quietly gone.
	it('reports entries the catalog no longer has', () => {
		render(
			<ScenariosPage
				{...props([
					{ ...gaming, launchIdentities: ['game', 'uninstalled'] },
				])}
			/>,
		)

		expect(
			screen.getByRole('list', { name: 'Launch list of Gaming' }),
		).toHaveTextContent('1 unavailable')
	})

	it('explains itself when there are no scenarios', () => {
		render(<ScenariosPage {...props()} />)

		expect(screen.getByText('No scenarios yet')).toBeVisible()
	})

	// Scenarios are stored in creation order, which would bury the one just made at the bottom.
	it('puts the newest scenario first and undated ones last', () => {
		render(
			<ScenariosPage
				{...props([
					{ ...gaming, id: 'legacy', name: 'Legacy', createdAt: null },
					{ ...gaming, id: 'older', name: 'Older', createdAt: 1_000 },
					{ ...gaming, id: 'newest', name: 'Newest', createdAt: 2_000 },
				])}
			/>,
		)

		expect(
			screen
				.getAllByRole('button', { name: /^Run / })
				.map(button => button.getAttribute('aria-label')),
		).toEqual(['Run Newest', 'Run Older', 'Run Legacy'])
	})
})
