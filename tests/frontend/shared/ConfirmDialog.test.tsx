import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ConfirmDialog } from '../../../src/shared/ui/ConfirmDialog'

function setup(props: Partial<Parameters<typeof ConfirmDialog>[0]> = {}) {
	const onConfirm = vi.fn()
	const onClose = vi.fn()
	render(
		<ConfirmDialog
			label="Delete Gaming scenario"
			title="Delete Gaming?"
			description="The scenario and its lists are removed."
			confirmLabel="Delete scenario"
			closeLabel="Close scenario deletion"
			onConfirm={onConfirm}
			onClose={onClose}
			{...props}
		/>,
	)
	return { onConfirm, onClose }
}

describe('ConfirmDialog', () => {
	it('is an alert dialog that opens on its safe control', () => {
		setup()

		const dialog = screen.getByRole('alertdialog', {
			name: 'Delete Gaming scenario',
		})
		expect(dialog).toHaveAttribute('aria-modal', 'true')
		expect(dialog).toHaveTextContent('Delete Gaming?')
		expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus()
	})

	// Every confirmation in the app now dismisses only through a control the reader aimed at.
	// Escape and a stray click on the backdrop used to throw the dialog away mid-read.
	it('survives Escape and a backdrop click', async () => {
		const { onClose, onConfirm } = setup()

		await userEvent.keyboard('{Escape}')
		await userEvent.click(
			screen.getByRole('alertdialog').parentElement as HTMLElement,
		)

		expect(onClose).not.toHaveBeenCalled()
		expect(onConfirm).not.toHaveBeenCalled()
		expect(screen.getByRole('alertdialog')).toBeInTheDocument()
	})

	it('cancels through Cancel and through the close control', async () => {
		const { onClose, onConfirm } = setup()

		await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
		await userEvent.click(
			screen.getByRole('button', { name: 'Close scenario deletion' }),
		)

		expect(onClose).toHaveBeenCalledTimes(2)
		expect(onConfirm).not.toHaveBeenCalled()
	})

	it('confirms through the named action', async () => {
		const { onConfirm } = setup()

		await userEvent.click(
			screen.getByRole('button', { name: 'Delete scenario' }),
		)

		expect(onConfirm).toHaveBeenCalledTimes(1)
	})

	it('locks every control while the confirmed work runs', () => {
		setup({ pending: true })

		for (const name of [
			'Cancel',
			'Delete scenario',
			'Close scenario deletion',
		])
			expect(screen.getByRole('button', { name })).toBeDisabled()
	})

	it('holds the action back while the detail block cannot vouch for it', () => {
		setup({ confirmDisabled: true, children: <p>Loading details…</p> })

		expect(
			screen.getByRole('button', { name: 'Delete scenario' }),
		).toBeDisabled()
		expect(screen.getByRole('button', { name: 'Cancel' })).toBeEnabled()
		expect(screen.getByText('Loading details…')).toBeVisible()
	})

	it('gives focus back to the control that opened it', async () => {
		render(<button type="button">Delete Gaming</button>)
		const trigger = screen.getByRole('button', { name: 'Delete Gaming' })
		trigger.focus()

		const view = render(
			<ConfirmDialog
				label="Delete Gaming scenario"
				title="Delete Gaming?"
				description="The scenario and its lists are removed."
				confirmLabel="Delete scenario"
				closeLabel="Close scenario deletion"
				onConfirm={vi.fn()}
				onClose={vi.fn()}
			/>,
		)
		expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus()

		view.unmount()
		expect(trigger).toHaveFocus()
	})
})
