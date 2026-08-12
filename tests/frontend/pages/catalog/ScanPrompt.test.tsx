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
})
