import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useIconRecovery } from '../../hooks/useIconRecovery'

describe('useIconRecovery', () => {
	afterEach(() => vi.useRealTimers())

	it('recovers missing icons every three hours while mounted', () => {
		vi.useFakeTimers()
		const recover = vi.fn().mockResolvedValue(undefined)
		renderHook(() => useIconRecovery(recover))

		act(() => vi.advanceTimersByTime(3 * 60 * 60 * 1000))

		expect(recover).toHaveBeenCalledOnce()
	})

	it('stops recovery after unmount', () => {
		vi.useFakeTimers()
		const recover = vi.fn().mockResolvedValue(undefined)
		const { unmount } = renderHook(() => useIconRecovery(recover))
		unmount()

		act(() => vi.advanceTimersByTime(3 * 60 * 60 * 1000))

		expect(recover).not.toHaveBeenCalled()
	})

	it('swallows a failed background recovery', async () => {
		vi.useFakeTimers()
		const recover = vi.fn().mockRejectedValue(new Error('unavailable'))
		renderHook(() => useIconRecovery(recover))

		act(() => vi.advanceTimersByTime(3 * 60 * 60 * 1000))
		await Promise.resolve()

		expect(recover).toHaveBeenCalledOnce()
	})
})
