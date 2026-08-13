import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { AppErrorBoundary } from '../../../src/app/AppErrorBoundary'

function BrokenScreen(): never {
	throw new Error('render failure')
}

describe('AppErrorBoundary', () => {
	it('shows a recovery screen instead of leaving a blank window', () => {
		const consoleError = vi
			.spyOn(console, 'error')
			.mockImplementation(() => undefined)
		const suppressError = (event: ErrorEvent) => event.preventDefault()
		window.addEventListener('error', suppressError)
		try {
			render(
				<AppErrorBoundary>
					<BrokenScreen />
				</AppErrorBoundary>,
			)

			expect(screen.getByRole('alert')).toHaveTextContent(
				'Something went wrong',
			)
			expect(
				screen.getByRole('button', { name: 'Reload app' }),
			).toBeEnabled()
		} finally {
			window.removeEventListener('error', suppressError)
			consoleError.mockRestore()
		}
	})
})
