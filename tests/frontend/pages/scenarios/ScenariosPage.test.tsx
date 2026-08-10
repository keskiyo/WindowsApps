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

const explorer: AppInfo = {
	...app('explorer', 'Проводник'),
	closeRisk: 'close.session',
}

const catalog = [app('game', 'Backpack Battles'), app('chat', 'Discord')]
const catalogWithShell = [...catalog, explorer]

function props(scenarios: Scenario[] = [], favoriteScenarioIds: string[] = []) {
	return {
		scenarios,
		apps: catalog,
		runningId: null,
		favoriteScenarioIds,
		onToggleFavorite: vi.fn(),
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

	it('shows the current operation count for the running scenario', () => {
		render(
			<ScenariosPage
				{...props([gaming])}
				runningId="gaming"
				runProgress={{ phase: 'launching', completed: 1, total: 2 }}
			/>,
		)

		expect(screen.getByText('Launching 1/2')).toBeInTheDocument()
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

	// The badge has to name the consequence, not the category: "Windows component" told the reader
	// nothing about what closing it would do, and said the same thing about an entry that is merely
	// impossible to close as about one that ends the desktop.
	it('labels a shell-ending entry and an unclosable one differently', async () => {
		const view = {
			...props([gaming]),
			apps: [
				...catalogWithShell,
				{
					...app('run', 'Выполнить'),
					closeRisk: 'close.not_closable',
				} as AppInfo,
			],
		}
		render(<ScenariosPage {...view} />)

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Add an app to the Close list of Gaming',
			}),
		)
		const picker = screen.getByRole('dialog', {
			name: 'Add an app to the close list of Gaming',
		})

		expect(
			within(picker).getByRole('button', { name: /Проводник/ }),
		).toHaveTextContent('Danger')
		expect(
			within(picker).getByRole('button', { name: /Выполнить/ }),
		).toHaveTextContent('Cannot close')
		expect(
			within(picker).getByRole('button', { name: /Discord/ }),
		).not.toHaveTextContent(/Danger|Cannot close/)
	})

	// Closing the shell is recoverable but never something to trigger by accident, so it costs one
	// deliberate confirmation and nothing reaches the scenario until that is given.
	it('asks before putting a Windows desktop component in the close list', async () => {
		const view = { ...props([gaming]), apps: catalogWithShell }
		render(<ScenariosPage {...view} />)

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Add an app to the Close list of Gaming',
			}),
		)
		await userEvent.click(
			within(
				screen.getByRole('dialog', {
					name: 'Add an app to the close list of Gaming',
				}),
			).getByRole('button', { name: /Проводник/ }),
		)

		expect(view.onAddApp).not.toHaveBeenCalled()
		const warning = screen.getByRole('alertdialog', {
			name: 'Add Проводник to the close list',
		})
		expect(warning).toHaveTextContent(/ends the shell/)

		await userEvent.click(
			within(warning).getByRole('button', { name: 'Add anyway' }),
		)
		expect(view.onAddApp).toHaveBeenCalledWith('gaming', 'close', 'explorer')
	})

	it('leaves the list untouched when the warning is dismissed', async () => {
		const view = { ...props([gaming]), apps: catalogWithShell }
		render(<ScenariosPage {...view} />)

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Add an app to the Close list of Gaming',
			}),
		)
		await userEvent.click(
			within(
				screen.getByRole('dialog', {
					name: 'Add an app to the close list of Gaming',
				}),
			).getByRole('button', { name: /Проводник/ }),
		)
		await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))

		expect(view.onAddApp).not.toHaveBeenCalled()
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
	})

	// Starting Explorer is ordinary; only ending it is not.
	it('adds the same component to the launch list without asking', async () => {
		const view = { ...props([gaming]), apps: catalogWithShell }
		render(<ScenariosPage {...view} />)

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Add an app to the Launch list of Gaming',
			}),
		)
		await userEvent.click(
			within(
				screen.getByRole('dialog', {
					name: 'Add an app to the launch list of Gaming',
				}),
			).getByRole('button', { name: /Проводник/ }),
		)

		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
		expect(view.onAddApp).toHaveBeenCalledWith('gaming', 'launch', 'explorer')
	})

	it('runs a scenario and blocks a second run while it is going', async () => {
		const view = { ...props([gaming]), runningId: 'gaming' }
		render(<ScenariosPage {...view} />)

		const run = screen.getByRole('button', { name: 'Run Gaming' })
		expect(run).toBeDisabled()
		expect(run).toHaveTextContent('Running…')
	})

	it('blocks the active scenario action until it finishes', () => {
		render(
			<ScenariosPage
				{...props([gaming])}
				runningId="gaming"
			/>,
		)

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

	it('stars a scenario and reports the one already starred', async () => {
		const view = props([gaming])
		const { rerender } = render(<ScenariosPage {...view} />)

		const star = screen.getByRole('button', {
			name: 'Add Gaming to favorites',
		})
		expect(star).toHaveAttribute('aria-pressed', 'false')
		await userEvent.click(star)
		expect(view.onToggleFavorite).toHaveBeenCalledWith('gaming')

		rerender(<ScenariosPage {...props([gaming], ['gaming'])} />)
		expect(
			screen.getByRole('button', { name: 'Remove Gaming from favorites' }),
		).toHaveAttribute('aria-pressed', 'true')
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
