import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useAppFeedback } from '../../hooks/useAppFeedback'
import type { AppInfo } from '../../types'

const { toastError } = vi.hoisted(() => ({ toastError: vi.fn() }))

vi.mock('sonner', () => ({
	toast: {
		error: toastError,
		info: vi.fn(),
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
	it('returns a failure result when uninstalling fails', async () => {
		const { result } = renderHook(() =>
			useAppFeedback({
				onLaunch: vi.fn().mockResolvedValue(undefined),
				onRefresh: vi.fn().mockResolvedValue(undefined),
				onUninstall: vi.fn().mockRejectedValue(new Error('private failure')),
			}),
		)

		let outcome: unknown
		await act(async () => {
			outcome = await result.current.uninstall(app)
		})

		expect(outcome).toEqual({ ok: false })
		expect(toastError).toHaveBeenCalledWith(
			'Could not uninstall Visual Studio Code',
		)
	})
})
