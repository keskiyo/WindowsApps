import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ScanPrompt } from '../../../../src/pages/catalog/ui/ScanPrompt'

describe('ScanPrompt', () => {
	it('explains that scanning builds the catalog and remains optional', async () => {
		const onDismiss = vi.fn()
		render(
			<ScanPrompt
				isScanning={false}
				onScan={vi.fn().mockResolvedValue(undefined)}
				onDismiss={onDismiss}
			/>,
		)

		expect(screen.getByText('Scan Windows to build your catalog. Nothing runs automatically at startup.')).toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('button', { name: 'Dismiss scan prompt' }),
		)
		expect(onDismiss).toHaveBeenCalledOnce()
	})

	// A first-run screen that only offers a button leaves the user guessing what will be read.
	it('names what the scan covers and how long it takes', () => {
		render(
			<ScanPrompt
				isScanning={false}
				onScan={vi.fn().mockResolvedValue(undefined)}
				onDismiss={vi.fn()}
			/>,
		)

		expect(
			screen.getByText(/Start Menu shortcuts and installed programs/),
		).toBeInTheDocument()
		expect(
			screen.getByText(/Microsoft Store apps and Steam games/),
		).toBeInTheDocument()
		expect(
			screen.getByText(/Portable executables in folders you choose/),
		).toBeInTheDocument()
		expect(screen.getByText(/under a minute/)).toBeInTheDocument()
	})

	it('offers the folder settings before the first scan', async () => {
		const onConfigureFolders = vi.fn()
		render(
			<ScanPrompt
				isScanning={false}
				onScan={vi.fn().mockResolvedValue(undefined)}
				onDismiss={vi.fn()}
				onConfigureFolders={onConfigureFolders}
			/>,
		)

		await userEvent.click(
			screen.getByRole('button', { name: 'Choose folders first' }),
		)

		expect(onConfigureFolders).toHaveBeenCalledOnce()
	})
})
