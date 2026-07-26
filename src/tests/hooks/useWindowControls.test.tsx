import { act, renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useWindowControls } from '../../hooks/useWindowControls'

const mocks = vi.hoisted(() => ({
	appWindow: {
		isMaximized: vi.fn(),
		onResized: vi.fn(),
		minimize: vi.fn(),
		toggleMaximize: vi.fn(),
		close: vi.fn(),
	},
}))

vi.mock('@tauri-apps/api/window', () => ({
	getCurrentWindow: () => mocks.appWindow,
}))

function deferredResized() {
	let settle: (stop: () => void) => void = () => undefined
	mocks.appWindow.onResized.mockReturnValue(
		new Promise<() => void>(resolve => {
			settle = resolve
		}),
	)
	return (stop: () => void) => settle(stop)
}

describe('useWindowControls', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		mocks.appWindow.isMaximized.mockResolvedValue(false)
		mocks.appWindow.minimize.mockResolvedValue(undefined)
		mocks.appWindow.toggleMaximize.mockResolvedValue(undefined)
		mocks.appWindow.close.mockResolvedValue(undefined)
		mocks.appWindow.onResized.mockResolvedValue(() => undefined)
	})

	it('unregisters a listener that resolves after unmount', async () => {
		const resolveResized = deferredResized()
		const { unmount } = renderHook(() => useWindowControls())
		unmount()

		// Registration completes only after cleanup already ran; the late handle must
		// still be disposed or the resize listener stays alive for the process lifetime.
		const stop = vi.fn()
		await act(async () => {
			resolveResized(stop)
		})

		expect(stop).toHaveBeenCalledOnce()
	})

	it('unregisters a listener that resolved before unmount', async () => {
		const stop = vi.fn()
		mocks.appWindow.onResized.mockResolvedValue(stop)
		const { unmount } = renderHook(() => useWindowControls())
		await waitFor(() => expect(mocks.appWindow.onResized).toHaveBeenCalled())

		unmount()

		expect(stop).toHaveBeenCalledOnce()
	})

	it('survives a registration that never succeeds', async () => {
		mocks.appWindow.onResized.mockRejectedValue(new Error('no window'))
		const { result, unmount } = renderHook(() => useWindowControls())

		await act(async () => undefined)

		expect(result.current.maximized).toBe(false)
		expect(() => unmount()).not.toThrow()
	})

	it('reflects the current maximized state', async () => {
		mocks.appWindow.isMaximized.mockResolvedValue(true)
		const { result } = renderHook(() => useWindowControls())

		await waitFor(() => expect(result.current.maximized).toBe(true))
	})

	it('drives the window through its controls', () => {
		const { result } = renderHook(() => useWindowControls())

		result.current.minimize()
		result.current.toggleMaximize()
		result.current.close()

		expect(mocks.appWindow.minimize).toHaveBeenCalledOnce()
		expect(mocks.appWindow.toggleMaximize).toHaveBeenCalledOnce()
		expect(mocks.appWindow.close).toHaveBeenCalledOnce()
	})
})
