import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ScenarioRunDialog } from '../../../../src/features/manage-scenarios'
import type { AppInfo } from '../../../../src/entities/app'
import type { Scenario } from '../../../../src/entities/scenario'

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

function scenario(value: Partial<Scenario> & Pick<Scenario, 'id'>): Scenario {
	return {
		name: value.id,
		launchIdentities: [],
		closeIdentities: [],
		createdAt: null,
		...value,
	}
}

const gaming = scenario({
	id: 'gaming',
	name: 'Gaming',
	launchIdentities: ['game'],
	closeIdentities: ['chat', 'mail'],
})

function renderDialog(value: {
	scenarios?: Scenario[]
	apps?: AppInfo[]
	runningId?: string | null
	isScenarioRunning?: boolean
	withCancel?: boolean
} = {}) {
	const onRun = vi.fn()
	const onClose = vi.fn()
	const onCancel = vi.fn()
	render(
		<ScenarioRunDialog
			scenarios={value.scenarios ?? [gaming]}
			apps={
				value.apps ?? [
					app('game', 'Backpack Battles'),
					app('chat', 'Chat'),
					app('mail', 'Mail'),
				]
		}
			runningId={value.runningId ?? null}
			isScenarioRunning={value.isScenarioRunning ?? false}
			onRun={onRun}
			onCancel={value.withCancel ? onCancel : undefined}
			onClose={onClose}
		/>,
	)
	return { onRun, onClose, onCancel }
}

describe('ScenarioRunDialog', () => {
	it('stops the scenario it is running', async () => {
		const { onCancel } = renderDialog({
			runningId: 'gaming',
			isScenarioRunning: true,
			withCancel: true,
		})

		await userEvent.click(screen.getByRole('button', { name: 'Stop Gaming' }))

		expect(onCancel).toHaveBeenCalledOnce()
	})

	it('is a modal dialog over the page', () => {
		renderDialog()

		const dialog = screen.getByRole('dialog', { name: 'All scenarios' })
		expect(dialog).toHaveAttribute('aria-modal', 'true')
	})

	// Picking a scenario to run is the point; its contents are what you open when unsure.
	it('lists scenarios collapsed, with the size of each list', () => {
		renderDialog()

		const toggle = screen.getByRole('button', { expanded: false })
		expect(toggle).toHaveTextContent('Gaming')
		expect(toggle).toHaveTextContent('1 launch · 2 close')
		expect(
			screen.queryByRole('list', { name: 'Launch list of Gaming' }),
		).not.toBeInTheDocument()
	})

	it('shows what a scenario starts and closes when its name is clicked', async () => {
		renderDialog()

		await userEvent.click(screen.getByRole('button', { expanded: false }))

		expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument()
		expect(
			screen.getByRole('list', { name: 'Launch list of Gaming' }),
		).toHaveTextContent('Backpack Battles')
		const close = screen.getByRole('list', { name: 'Close list of Gaming' })
		expect(close).toHaveTextContent('Chat')
		expect(close).toHaveTextContent('Mail')
	})

	it('collapses the scenario again on a second click', async () => {
		renderDialog()

		await userEvent.click(screen.getByRole('button', { expanded: false }))
		await userEvent.click(screen.getByRole('button', { expanded: true }))

		expect(
			screen.queryByRole('list', { name: 'Launch list of Gaming' }),
		).not.toBeInTheDocument()
	})

	// This dialog runs scenarios; an edit control here would be a way to break one by mistake.
	it('never offers to remove an app from a list', async () => {
		renderDialog()

		await userEvent.click(screen.getByRole('button', { expanded: false }))

		expect(screen.queryByRole('button', { name: /^Remove / })).toBeNull()
	})

	it('runs the scenario it was asked to run', async () => {
		const { onRun } = renderDialog()

		await userEvent.click(screen.getByRole('button', { name: 'Run Gaming' }))

		expect(onRun).toHaveBeenCalledWith('gaming')
	})

	// Stored order is creation order, which would bury the scenario just made at the bottom.
	it('puts the newest scenario first and undated ones last', () => {
		renderDialog({
			scenarios: [
				scenario({ id: 'legacy', name: 'Legacy' }),
				scenario({ id: 'older', name: 'Older', createdAt: 1_000 }),
				scenario({ id: 'newest', name: 'Newest', createdAt: 2_000 }),
			],
		})

		expect(
			screen
				.getAllByRole('button', { name: /^Run / })
				.map(button => button.getAttribute('aria-label')),
		).toEqual(['Run Newest', 'Run Older', 'Run Legacy'])
	})

	it('blocks every scenario action until the active run finishes', () => {
		renderDialog({
			scenarios: [gaming, scenario({ id: 'work', name: 'Work' })],
			runningId: 'gaming',
			isScenarioRunning: true,
		})

		const run = screen.getByRole('button', { name: 'Run Gaming' })
		expect(run).toBeDisabled()
		expect(run).toHaveTextContent('Running…')
		expect(
			screen.getByRole('button', {
				name: 'Run Work unavailable while another scenario is running',
			}),
		).toBeDisabled()
	})

	it('shows the saved names for entries the catalog no longer has', async () => {
		renderDialog({
			apps: [app('game', 'Backpack Battles')],
			scenarios: [
				scenario({
					...gaming,
					closeAppSnapshots: {
						chat: { name: 'Chat', iconBase64: null },
						mail: { name: 'Mail', iconBase64: null },
					},
				}),
			],
		})

		await userEvent.click(screen.getByRole('button', { expanded: false }))

		const close = screen.getByRole('list', { name: 'Close list of Gaming' })
		expect(close).toHaveTextContent('Chat')
		expect(close).toHaveTextContent('Mail')
		expect(close).toHaveTextContent('Unavailable')
		expect(screen.queryByRole('button', { name: /^Remove / })).toBeNull()
	})

	it('closes on Escape and on the close button', async () => {
		const { onClose } = renderDialog()

		await userEvent.keyboard('{Escape}')
		expect(onClose).toHaveBeenCalledOnce()

		await userEvent.click(
			screen.getByRole('button', { name: 'Close all scenarios' }),
		)
		expect(onClose).toHaveBeenCalledTimes(2)
	})

	it('starts with the keyboard inside the dialog', () => {
		renderDialog()

		expect(
			screen.getByRole('button', { name: 'Close all scenarios' }),
		).toHaveFocus()
	})
})
