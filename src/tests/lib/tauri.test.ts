import { beforeEach, describe, expect, it, vi } from 'vitest'

const invokeMock = vi.fn()
const listenMock = vi.fn()

vi.mock('@tauri-apps/api/core', () => ({
	invoke: invokeMock,
}))

vi.mock('@tauri-apps/api/event', () => ({
	listen: listenMock,
}))

describe('tauri app client browser fallback', () => {
	beforeEach(() => {
		vi.resetModules()
		invokeMock.mockReset()
		listenMock.mockReset()
		delete (globalThis as { __TAURI_INTERNALS__?: unknown })
			.__TAURI_INTERNALS__
	})

	it('does not call Tauri IPC when opened in a regular browser', async () => {
		const { tauriAppsClient } = await import('../../lib/tauri')

		await expect(tauriAppsClient.getApps()).resolves.toEqual({
			apps: [],
			hasCache: false,
		})
		await expect(
			tauriAppsClient.onCatalogDelta?.(() => undefined),
		).resolves.toEqual(expect.any(Function))

		expect(invokeMock).not.toHaveBeenCalled()
		expect(listenMock).not.toHaveBeenCalled()
	})

	it('preserves structured backend error codes and hides unknown transport details', async () => {
		const { toAppClientError } = await import('../../lib/tauri')
		expect(
			toAppClientError({
				code: 'LAUNCH_UNAVAILABLE',
				message: 'This application is not available for launch.',
			}),
		).toMatchObject({
			code: 'LAUNCH_UNAVAILABLE',
			message: 'This application is not available for launch.',
		})
		expect(
			toAppClientError(new Error('C:\\Users\\Maks\\private-detail')),
		).toMatchObject({
			code: 'INTERNAL',
			message: 'The operation could not be completed. Try again.',
		})
	})
})
