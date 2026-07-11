import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import {
	releaseHighlights,
	UpdateDialog,
} from '../../../components/shared/UpdateDialog'

describe('releaseHighlights', () => {
	it('extracts the Highlights section from GitHub release markdown', () => {
		const notes = [
			'## Highlights',
			'',
			'- **Automatic updates** - the app checks on startup.',
			'- `Quick launch` - keyboard-first command palette.',
			'- Better search - matches name and publisher.',
			'',
			'## Fixes',
			'- This should not be included.',
		].join('\n')

		expect(releaseHighlights(notes)).toEqual([
			'Automatic updates - the app checks on startup.',
			'Quick launch - keyboard-first command palette.',
			'Better search - matches name and publisher.',
		])
	})

	it('falls back to the first bullets when there is no Highlights heading', () => {
		expect(releaseHighlights('- One\n- Two\n- Three\n- Four\n- Five')).toEqual(
			['One', 'Two', 'Three', 'Four'],
		)
	})
})

describe('UpdateDialog', () => {
	it('shows release highlights and lets the user install or dismiss', async () => {
		const onInstall = vi.fn()
		const onDismiss = vi.fn()

		render(
			<UpdateDialog
				version='0.2.2'
				notes={'## Highlights\n- **Automatic updates** - signed update.'}
				installing={false}
				progress={null}
				onInstall={onInstall}
				onDismiss={onDismiss}
			/>,
		)

		expect(
			screen.getByRole('dialog', { name: 'Update 0.2.2 available' }),
		).toBeInTheDocument()
		expect(
			screen.getByText('Automatic updates - signed update.'),
		).toBeInTheDocument()

		await userEvent.click(
			screen.getByRole('button', { name: 'Update & restart' }),
		)
		expect(onInstall).toHaveBeenCalledTimes(1)

		await userEvent.click(screen.getByRole('button', { name: 'Later' }))
		expect(onDismiss).toHaveBeenCalledTimes(1)
	})

	it('traps focus, closes on Escape, and restores focus', async () => {
		const user = userEvent.setup()
		const onDismiss = vi.fn()
		const { rerender } = render(
			<button type='button'>Open updates</button>,
		)
		const opener = screen.getByRole('button', { name: 'Open updates' })
		opener.focus()
		rerender(
			<>
				<button type='button'>Open updates</button>
				<UpdateDialog
					version='0.2.2'
					notes={'## Highlights\n- Safer update window.'}
					installing={false}
					progress={null}
					onInstall={vi.fn()}
					onDismiss={onDismiss}
				/>
			</>,
		)

		const close = screen.getByRole('button', { name: 'Dismiss update' })
		expect(close).toHaveFocus()

		await user.keyboard('{Shift>}{Tab}{/Shift}')
		expect(
			screen.getByRole('button', { name: 'Update & restart' }),
		).toHaveFocus()

		await user.keyboard('{Escape}')
		expect(onDismiss).toHaveBeenCalledTimes(1)
		rerender(<button type='button'>Open updates</button>)
		expect(
			screen.getByRole('button', { name: 'Open updates' }),
		).toHaveFocus()
	})

	it('does not dismiss with Escape while installing', async () => {
		const onDismiss = vi.fn()
		render(
			<UpdateDialog
				version='0.2.2'
				notes={'## Highlights\n- Safer update window.'}
				installing
				progress={42}
				onInstall={vi.fn()}
				onDismiss={onDismiss}
			/>,
		)

		await userEvent.keyboard('{Escape}')
		expect(onDismiss).not.toHaveBeenCalled()
		expect(screen.getByLabelText('Update progress')).toBeInTheDocument()
	})
})
