import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { UpdateDialog } from '../../../components/shared/UpdateDialog'
import { releaseHighlights } from '../../../components/shared/releaseHighlights'

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
		const onOpenRelease = vi.fn()

		render(
			<UpdateDialog
				version='0.2.2'
				date='2026-07-11T10:00:00Z'
				packageSize={5_600_000}
				releaseUrl='https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.2'
				notes={'## Highlights\n- **Automatic updates** - signed update.'}
				installing={false}
				progress={null}
				downloadedBytes={0}
				totalBytes={null}
				phase='idle'
				error={null}
				onInstall={onInstall}
				onDismiss={onDismiss}
				onOpenRelease={onOpenRelease}
			/>,
		)

		expect(
			screen.getByRole('dialog', { name: 'Update 0.2.2 available' }),
		).toBeInTheDocument()
		expect(
			screen.getByText('Automatic updates - signed update.'),
		).toBeInTheDocument()
		expect(screen.getByText('11 Jul 2026')).toBeInTheDocument()
		expect(screen.getByText('5.3 MB')).toBeInTheDocument()
		expect(
			screen.getByRole('link', { name: 'View full release notes' }),
		).toHaveAttribute(
			'href',
			'https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.2',
		)
		await userEvent.click(
			screen.getByRole('link', { name: 'View full release notes' }),
		)
		expect(onOpenRelease).toHaveBeenCalledOnce()

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
					date={null}
					packageSize={null}
					releaseUrl={null}
					notes={'## Highlights\n- Safer update window.'}
					installing={false}
					progress={null}
					downloadedBytes={0}
					totalBytes={null}
					phase='idle'
					error={null}
					onInstall={vi.fn()}
					onDismiss={onDismiss}
					onOpenRelease={vi.fn()}
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
				date={null}
				packageSize={null}
				releaseUrl={null}
				notes={'## Highlights\n- Safer update window.'}
				installing
				progress={42}
				downloadedBytes={2_400_000}
				totalBytes={5_600_000}
				phase='downloading'
				error={null}
				onInstall={vi.fn()}
				onDismiss={onDismiss}
				onOpenRelease={vi.fn()}
			/>,
		)

		await userEvent.keyboard('{Escape}')
		expect(onDismiss).not.toHaveBeenCalled()
		expect(screen.getByLabelText('Update progress')).toBeInTheDocument()
		expect(screen.getByText('Downloading')).toBeInTheDocument()
		expect(screen.getByLabelText('2.3 MB of 5.3 MB')).toBeInTheDocument()
	})

	it('uses quiet update wording during the install phase', () => {
		render(
			<UpdateDialog
				version='0.2.4'
				date={null}
				packageSize={null}
				releaseUrl={null}
				notes={'## Highlights\n- Quiet updater.'}
				installing
				progress={100}
				downloadedBytes={5_600_000}
				totalBytes={5_600_000}
				phase='installing'
				error={null}
				onInstall={vi.fn()}
				onDismiss={vi.fn()}
				onOpenRelease={vi.fn()}
			/>,
		)

		expect(
			screen.getByRole('button', { name: 'Finishing update...' }),
		).toBeInTheDocument()
		expect(screen.queryByText('Installing...')).not.toBeInTheDocument()
	})

	it('keeps the dialog open with a retry action after an install error', async () => {
		const onInstall = vi.fn()
		render(
			<UpdateDialog
				version='0.2.3'
				date={null}
				packageSize={null}
				releaseUrl={null}
				notes={'## Highlights\n- Reliable updates.'}
				installing={false}
				progress={null}
				downloadedBytes={0}
				totalBytes={null}
				phase='failed'
				error='The update package is unavailable.'
				onInstall={onInstall}
				onDismiss={vi.fn()}
				onOpenRelease={vi.fn()}
			/>,
		)

		expect(screen.getByRole('alert')).toHaveTextContent(
			'The update package is unavailable.',
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Retry update' }),
		)
		expect(onInstall).toHaveBeenCalledOnce()
	})
})
