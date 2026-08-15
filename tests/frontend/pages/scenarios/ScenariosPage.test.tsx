import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ScenariosPage } from '../../../../src/pages/scenarios'
import type { Scenario } from '../../../../src/entities/scenario'
import type { AppInfo } from '../../../../src/entities/app'
import { DEFAULT_CATEGORIES } from '../../../../src/entities/category'

function app(id: string, name: string, category = 'other'): AppInfo {
	return {
		id,
		name,
		path: `C:\\Apps\\${id}.exe`,
		category,
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

const catalog = [
	app('game', 'Backpack Battles', 'games'),
	app('chat', 'Discord', 'communication'),
]
const catalogWithShell = [...catalog, explorer]

function props(scenarios: Scenario[] = [], favoriteScenarioIds: string[] = []) {
	return {
		scenarios,
		apps: catalog,
		selectableApps: catalog,
		categories: DEFAULT_CATEGORIES,
		runningId: null,
		isScenarioRunning: false,
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

const work: Scenario = {
	id: 'work',
	name: 'Work',
	launchIdentities: ['chat'],
	closeIdentities: [],
	createdAt: null,
}

const empty: Scenario = {
	id: 'empty',
	name: 'Empty',
	launchIdentities: [],
	closeIdentities: [],
	createdAt: null,
}

const unclosable: AppInfo = {
	...app('run', 'Выполнить'),
	closeRisk: 'close.not_closable',
}

async function openPicker(list: 'Launch' | 'Close', scenario: string) {
	await userEvent.click(
		screen.getByRole('button', {
			name: `Add an app to the ${list} list of ${scenario}`,
		}),
	)
	return screen.getByRole('dialog', {
		name: `Add an app to the ${list.toLowerCase()} list of ${scenario}`,
	})
}

async function check(picker: HTMLElement, name: RegExp) {
	await userEvent.click(within(picker).getByRole('switch', { name }))
}

async function confirmPicker(picker: HTMLElement) {
	await userEvent.click(
		within(picker).getByRole('button', { name: 'Add selected apps' }),
	)
}

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

		const picker = await openPicker('Close', 'Gaming')
		await check(picker, /Discord/)
		await confirmPicker(picker)

		expect(view.onAddApp).toHaveBeenCalledWith('gaming', 'close', 'chat')
		expect(
			screen.queryByRole('dialog', { name: /Add an app/ }),
		).not.toBeInTheDocument()
	})

	// Adding one app per trip through the dialog was the whole complaint: a scenario is built from
	// several apps at once, so the dialog closes once and the whole checked set goes in.
	it('adds every checked app in a single pass', async () => {
		const view = props([empty])
		render(<ScenariosPage {...view} />)

		const picker = await openPicker('Launch', 'Empty')
		await check(picker, /Backpack Battles/)
		await check(picker, /Discord/)
		await confirmPicker(picker)

		expect(view.onAddApp.mock.calls).toEqual([
			['empty', 'launch', 'game'],
			['empty', 'launch', 'chat'],
		])
	})

	// The category is the context that makes a name recognisable; the picker reads it from the
	// catalog rather than asking the reader to remember.
	it('shows the category of every offered app', async () => {
		render(<ScenariosPage {...props([empty])} />)

		const picker = await openPicker('Launch', 'Empty')

		expect(
			within(picker).getByRole('switch', { name: /Backpack Battles/ }),
		).toHaveAccessibleName(/Games/)
		expect(
			within(picker).getByRole('switch', { name: /Discord/ }),
		).toHaveAccessibleName(/Communication/)
	})

	// The picker offered the whole catalog, tools and Windows components included, so it listed
	// far more entries than All apps ever shows. It now offers exactly that view, while an entry
	// already sitting in a scenario still resolves against the wider catalog.
	it('offers only what All apps shows and still resolves the rest', async () => {
		const tool: AppInfo = {
			...app('backup', 'Архивация Windows', 'windows_features'),
			visibilityClass: 'auxiliary',
		}
		render(
			<ScenariosPage
				{...props([{ ...empty, launchIdentities: ['backup'] }])}
				apps={[...catalog, tool]}
				selectableApps={catalog}
			/>,
		)

		expect(
			screen.getByRole('list', { name: 'Launch list of Empty' }),
		).toHaveTextContent('Архивация Windows')
		const picker = await openPicker('Close', 'Empty')
		expect(
			within(picker).queryByRole('switch', { name: /Архивация Windows/ }),
		).not.toBeInTheDocument()
		expect(within(picker).getAllByRole('switch')).toHaveLength(2)
	})

	it('leaves out an app the list already holds', async () => {
		render(<ScenariosPage {...props([gaming])} />)

		const picker = await openPicker('Launch', 'Gaming')

		expect(
			within(picker).queryByRole('switch', { name: /Backpack Battles/ }),
		).not.toBeInTheDocument()
		expect(
			within(picker).getByRole('switch', { name: /Discord/ }),
		).toBeEnabled()
	})

	// The store refuses an app that both launches and closes; the dialog says so instead of
	// letting the user check a box that turns into an error.
	it('locks an app that already sits in the other list', async () => {
		render(<ScenariosPage {...props([gaming])} />)

		const picker = await openPicker('Close', 'Gaming')
		const locked = within(picker).getByRole('switch', {
			name: /Backpack Battles/,
		})

		expect(locked).toBeDisabled()
		expect(locked).toHaveAccessibleName(/Already in the launch list/)
	})

	// The badge has to name the consequence, not the category: "Windows component" told the reader
	// nothing about what closing it would do, and said the same thing about an entry that is merely
	// impossible to close as about one that ends the desktop.
	it('labels a shell-ending entry and an unclosable one differently', async () => {
		const view = {
			...props([gaming]),
			apps: [...catalogWithShell, unclosable],
			selectableApps: [...catalogWithShell, unclosable],
		}
		render(<ScenariosPage {...view} />)

		const picker = await openPicker('Close', 'Gaming')

		expect(
			within(picker).getByRole('switch', { name: /Проводник/ }),
		).toHaveAccessibleName(/Danger/)
		expect(
			within(picker).getByRole('switch', { name: /Выполнить/ }),
		).toHaveAccessibleName(/Cannot close/)
		expect(
			within(picker).getByRole('switch', { name: /Discord/ }),
		).not.toHaveAccessibleName(/Danger|Cannot close/)
	})

	// Closing the shell is recoverable but never something to trigger by accident, so it costs one
	// deliberate confirmation and nothing reaches the scenario until that is given.
	it('asks before putting a Windows desktop component in the close list', async () => {
		const view = { ...props([gaming]), apps: catalogWithShell, selectableApps: catalogWithShell }
		render(<ScenariosPage {...view} />)

		const picker = await openPicker('Close', 'Gaming')
		await check(picker, /Проводник/)
		await confirmPicker(picker)

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
		const view = { ...props([gaming]), apps: catalogWithShell, selectableApps: catalogWithShell }
		render(<ScenariosPage {...view} />)

		const picker = await openPicker('Close', 'Gaming')
		await check(picker, /Проводник/)
		await confirmPicker(picker)
		await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))

		expect(view.onAddApp).not.toHaveBeenCalled()
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
	})

	// One confirmation per risky app: checking several at once must not turn a single click into
	// blanket consent for all of them.
	it('asks once per risky app and adds the safe ones straight away', async () => {
		const view = {
			...props([empty]),
			apps: [...catalogWithShell, unclosable],
			selectableApps: [...catalogWithShell, unclosable],
		}
		render(<ScenariosPage {...view} />)

		const picker = await openPicker('Close', 'Empty')
		await check(picker, /Discord/)
		await check(picker, /Проводник/)
		await check(picker, /Выполнить/)
		await confirmPicker(picker)

		expect(view.onAddApp.mock.calls).toEqual([['empty', 'close', 'chat']])
		await userEvent.click(
			within(
				screen.getByRole('alertdialog', {
					name: 'Add Проводник to the close list',
				}),
			).getByRole('button', { name: 'Add anyway' }),
		)
		await userEvent.click(
			within(
				screen.getByRole('alertdialog', {
					name: 'Add Выполнить to the close list',
				}),
			).getByRole('button', { name: 'Cancel' }),
		)

		expect(view.onAddApp.mock.calls).toEqual([
			['empty', 'close', 'chat'],
			['empty', 'close', 'explorer'],
		])
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
	})

	// Starting Explorer is ordinary; only ending it is not.
	it('adds the same component to the launch list without asking', async () => {
		const view = { ...props([gaming]), apps: catalogWithShell, selectableApps: catalogWithShell }
		render(<ScenariosPage {...view} />)

		const picker = await openPicker('Launch', 'Gaming')
		await check(picker, /Проводник/)
		await confirmPicker(picker)

		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
		expect(view.onAddApp).toHaveBeenCalledWith('gaming', 'launch', 'explorer')
	})

	// Delete sits next to Rename and wipes a configured scenario outright, so a mis-click on the
	// wrong icon used to cost the whole thing with no way back.
	it('asks before deleting a scenario and does nothing when refused', async () => {
		const view = props([gaming])
		render(<ScenariosPage {...view} />)

		await userEvent.click(screen.getByRole('button', { name: 'Delete Gaming' }))

		const confirm = screen.getByRole('alertdialog', {
			name: 'Delete Gaming scenario',
		})
		expect(view.onDelete).not.toHaveBeenCalled()

		await userEvent.click(within(confirm).getByRole('button', { name: 'Cancel' }))
		expect(view.onDelete).not.toHaveBeenCalled()
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
	})

	it('deletes the scenario once the confirmation is given', async () => {
		const view = props([gaming])
		render(<ScenariosPage {...view} />)

		await userEvent.click(screen.getByRole('button', { name: 'Delete Gaming' }))
		await userEvent.click(
			screen.getByRole('button', { name: 'Delete scenario' }),
		)

		expect(view.onDelete).toHaveBeenCalledWith('gaming')
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
	})

	// Rename sat between the star and Run, so the primary action was the one control that moved
	// as the header grew. It now anchors the row and the pencil follows it.
	it('keeps the rename control to the right of the run button', () => {
		render(<ScenariosPage {...props([gaming])} />)

		const labels = within(screen.getByRole('region', { name: 'Gaming' }))
			.getAllByRole('button')
			.map(button => button.getAttribute('aria-label'))

		expect(labels.indexOf('Run Gaming')).toBeLessThan(
			labels.indexOf('Rename Gaming'),
		)
		expect(labels.indexOf('Rename Gaming')).toBeLessThan(
			labels.indexOf('Delete Gaming'),
		)
	})

	it('runs a scenario and blocks a second run while it is going', async () => {
		const view = {
			...props([gaming]),
			runningId: 'gaming',
			isScenarioRunning: true,
		}
		render(<ScenariosPage {...view} />)

		const run = screen.getByRole('button', { name: 'Run Gaming' })
		expect(run).toBeDisabled()
		expect(run).toHaveTextContent('Running…')
	})

	it('blocks every scenario action until the active run finishes', () => {
		render(
			<ScenariosPage
				{...props([gaming, work])}
				runningId="gaming"
				isScenarioRunning
			/>,
		)

		const run = screen.getByRole('button', { name: 'Run Gaming' })
		expect(run).toBeDisabled()
		expect(
			screen.getByRole('button', {
				name: 'Run Work unavailable while another scenario is running',
			}),
		).toBeDisabled()
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

	it('locks the active scenario configuration without locking its favorite control', () => {
		render(
			<ScenariosPage
				{...props([gaming], ['gaming'])}
				runningId="gaming"
				isScenarioRunning
			/>,
		)

		expect(screen.getByRole('button', { name: 'Rename Gaming' })).toBeDisabled()
		expect(screen.getByRole('button', { name: 'Delete Gaming' })).toBeDisabled()
		expect(
			screen.getByRole('button', {
				name: 'Add an app to the Launch list of Gaming',
			}),
		).toBeDisabled()
		expect(
			screen.getByRole('button', {
				name: 'Remove Backpack Battles from the Launch list of Gaming',
			}),
		).toBeDisabled()
		expect(
			screen.getByRole('button', {
				name: 'Remove Gaming from favorites',
			}),
		).toBeEnabled()
	})

	// An entry that stopped resolving must be visible, not quietly gone.
	it('shows a removable fallback for a legacy unavailable entry', () => {
		render(
			<ScenariosPage
				{...props([
					{ ...gaming, launchIdentities: ['game', 'uninstalled'] },
				])}
			/>,
		)

		const list = screen.getByRole('list', { name: 'Launch list of Gaming' })
		expect(list).toHaveTextContent('Unavailable application')
		expect(list).toHaveTextContent('Unavailable')
		expect(
			screen.getByRole('button', {
				name: 'Remove Unavailable application from the Launch list of Gaming',
			}),
		).toBeEnabled()
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
