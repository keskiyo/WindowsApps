import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AppShellChrome } from '../../../src/app/layout/AppShellChrome'
import type { AppShellChromeProps } from '../../../src/app/types'

function chrome(overrides: Partial<AppShellChromeProps['updater']> = {}) {
	const dismiss = vi.fn()
	const install = vi.fn().mockResolvedValue(undefined)
	const props: AppShellChromeProps = {
		activityActive: false,
		activityLabel: '',
		preferencesPersisted: true,
		staleCopy: null,
		systemClient: {
			openGithub: vi.fn().mockResolvedValue(undefined),
			openInstalledCopy: vi.fn().mockResolvedValue(undefined),
			openRelease: vi.fn().mockResolvedValue(undefined),
		},
		updater: {
			update: {
				version: '0.3.5',
				notes: null,
				date: null,
				packageSize: null,
				releaseUrl: null,
			},
			installing: false,
			progress: null,
			downloadedBytes: 0,
			totalBytes: null,
			phase: 'idle',
			error: null,
			status: 'available',
			checkNow: vi.fn().mockResolvedValue(undefined),
			install,
			dismiss,
			...overrides,
		},
		onDismissStaleCopy: vi.fn(),
	}
	render(<AppShellChrome {...props} />)
	return { dismiss, install }
}

// An update used to open a modal over the catalog before the user had done anything.
describe('AppShellChrome update surface', () => {
	it('announces an available update without taking over the window', () => {
		chrome()

		expect(
			screen.getByRole('button', { name: 'View update' }),
		).toBeInTheDocument()
		expect(screen.queryByRole('dialog')).toBeNull()
	})

	it('opens the details only when the user asks for them', async () => {
		chrome()

		await userEvent.click(
			screen.getByRole('button', { name: 'View update' }),
		)

		expect(screen.getByRole('dialog')).toBeInTheDocument()
	})

	it('remembers a dismissed version from the banner', async () => {
		const { dismiss } = chrome()

		await userEvent.click(
			screen.getByRole('button', { name: 'Dismiss update notice' }),
		)

		expect(dismiss).toHaveBeenCalledOnce()
	})

	it('keeps the dialog open while an install is running', () => {
		chrome({ installing: true, phase: 'downloading' })

		expect(screen.getByRole('dialog')).toBeInTheDocument()
		expect(screen.queryByRole('button', { name: 'View update' })).toBeNull()
	})
})
