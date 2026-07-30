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
		const { tauriAppsClient } = await import('../../../src/lib/tauri')

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
		const { toAppClientError } = await import('../../../src/lib/clientError')
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

	// The error-code set is a cross-language contract. The Rust half is pinned by
	// `error_codes_form_the_expected_stable_contract`; this pins the frontend half so the two
	// cannot drift silently: every backend code is recognized, plus exactly the two the frontend
	// owns. Adding a backend code without mirroring it here fails this test.
	it('mirrors the backend AppError code contract exactly', async () => {
		const { APP_ERROR_CODES } = await import('../../../src/lib/clientError')
		const backend = [
			'APP_DATA_UNAVAILABLE',
			'CLEAR_ICON_CACHE_FAILED',
			'CLEAR_UNINSTALL_HISTORY_FAILED',
			'INVALID_RELEASE_VERSION',
			'INVALID_HYDRATION_REQUEST',
			'LAUNCH_DATA_UNAVAILABLE',
			'LAUNCH_UNAVAILABLE',
			'NO_NEWER_COPY',
			'OPERATION_FAILED',
			'OPERATION_INTERRUPTED',
			'PRODUCT_NAME_MISSING',
			'RESET_CATALOG_CACHE_FAILED',
			'RESET_ICON_CACHE_FAILED',
			'SAVE_SCAN_SETTINGS_FAILED',
			'SCAN_CANCELLED',
			'SCAN_COALESCED',
			'SCAN_PATH_NOT_ABSOLUTE',
			'UNINSTALL_DATA_UNAVAILABLE',
			'UNINSTALL_UNAVAILABLE',
		]
		const frontendOnly = ['DESKTOP_RUNTIME_UNAVAILABLE', 'INTERNAL']
		expect(Object.keys(APP_ERROR_CODES).sort()).toEqual(
			[...backend, ...frontendOnly].sort(),
		)
	})
})
