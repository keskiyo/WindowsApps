import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { AppErrorBoundary } from '../../../src/app/AppErrorBoundary'

function BrokenScreen(): never {
	throw new Error('render failure')
}

function withSuppressedRenderError(work: () => void) {
	const consoleError = vi
		.spyOn(console, 'error')
		.mockImplementation(() => undefined)
	const suppressError = (event: ErrorEvent) => event.preventDefault()
	window.addEventListener('error', suppressError)
	try {
		work()
	} finally {
		window.removeEventListener('error', suppressError)
		consoleError.mockRestore()
	}
}

describe('AppErrorBoundary', () => {
	// A swallowed render failure left nothing to diagnose a user report with.
	it('reports the failure kind and stack to its owner', () => {
		const onError = vi.fn()
		withSuppressedRenderError(() => {
			render(
				<AppErrorBoundary onError={onError}>
					<BrokenScreen />
				</AppErrorBoundary>,
			)
		})

		expect(onError).toHaveBeenCalledOnce()
		const [kind, detail] = onError.mock.calls[0] as [string, string]
		expect(kind).toBe('Error')
		expect(detail).toContain('render failure')
	})

	it('replaces only its own subtree when a fallback is given', () => {
		withSuppressedRenderError(() => {
			render(
				<div>
					<p>Catalog stays</p>
					<AppErrorBoundary fallback={<p>Dialog closed</p>}>
						<BrokenScreen />
					</AppErrorBoundary>
				</div>,
			)
		})

		expect(screen.getByText('Catalog stays')).toBeInTheDocument()
		expect(screen.getByText('Dialog closed')).toBeInTheDocument()
		expect(screen.queryByRole('alert')).toBeNull()
	})

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
