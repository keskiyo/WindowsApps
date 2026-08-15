import { act, renderHook } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppFeedback } from '../../../src/app/model/useAppFeedback'
import type { AppInfo } from '../../../src/entities/app'

const { toastError, toastInfo } = vi.hoisted(() => ({
	toastError: vi.fn(),
	toastInfo: vi.fn(),
}))

vi.mock('sonner', () => ({
	toast: {
		error: toastError,
		info: toastInfo,
		success: vi.fn(),
	},
}))

const app: AppInfo = {
	id: 'code',
	name: 'Visual Studio Code',
	path: 'C:\\Code.exe',
	iconBase64: null,
	category: 'development',
	launchKind: 'executable',
	sourceKind: 'registry',
	description: null,
	version: null,
	publisher: null,
	installLocation: null,
	canUninstall: true,
}

describe('useAppFeedback', () => {
	beforeEach(() => {
		vi.clearAllMocks()
	})

	it('returns a failure result when uninstalling fails', async () => {
		const { result } = renderHook(() =>
			useAppFeedback({
				onLaunch: vi.fn().mockResolvedValue(undefined),
				onRefresh: vi.fn().mockResolvedValue(undefined),
				onUninstall: vi
					.fn()
					.mockRejectedValue(new Error('private failure')),
			}),
		)

		let outcome: unknown
		await act(async () => {
			outcome = await result.current.uninstall(app)
		})

		expect(outcome).toBe('failed')
		expect(toastError).toHaveBeenCalledWith(
			'Could not uninstall Visual Studio Code',
		)
	})

	// Closing the uninstall wizard is a decision. Reporting it as "could not uninstall" told the
	// user something had gone wrong when nothing had.
	it('reports a cancelled uninstall as informational, never as a failure', async () => {
		const { result } = renderHook(() =>
			useAppFeedback({
				onLaunch: vi.fn().mockResolvedValue(undefined),
				onRefresh: vi.fn().mockResolvedValue(undefined),
				onUninstall: vi.fn().mockRejectedValue({
					code: 'UNINSTALL_CANCELLED',
					message: 'The uninstall was cancelled.',
				}),
			}),
		)

		let outcome: unknown
		await act(async () => {
			outcome = await result.current.uninstall(app)
		})

		expect(outcome).toBe('cancelled')
		expect(toastInfo).toHaveBeenCalledWith(
			'Uninstall of Visual Studio Code cancelled',
		)
		expect(toastError).not.toHaveBeenCalled()
	})

	it('reports typed scan cancellation as informational feedback', async () => {
		const { result } = renderHook(() =>
			useAppFeedback({
				onLaunch: vi.fn().mockResolvedValue(undefined),
				onRefresh: vi.fn().mockRejectedValue({
					code: 'SCAN_CANCELLED',
					message: 'Application scan cancelled.',
				}),
				onUninstall: vi.fn().mockResolvedValue(undefined),
			}),
		)

		await act(() => result.current.refresh())

		expect(toastInfo).toHaveBeenCalledWith('Application scan cancelled')
		expect(toastError).not.toHaveBeenCalled()
	})

	it('does not infer scan cancellation from an untyped error message', async () => {
		const { result } = renderHook(() =>
			useAppFeedback({
				onLaunch: vi.fn().mockResolvedValue(undefined),
				onRefresh: vi
					.fn()
					.mockRejectedValue(new Error('scan cancelled internally')),
				onUninstall: vi.fn().mockResolvedValue(undefined),
			}),
		)

		await act(() => result.current.refresh())

		expect(toastError).toHaveBeenCalledWith(
			'Could not refresh the application list',
		)
		expect(toastInfo).not.toHaveBeenCalled()
	})
})
