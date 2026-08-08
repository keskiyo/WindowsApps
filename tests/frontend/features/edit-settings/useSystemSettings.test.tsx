import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useSystemSettings } from '../../../../src/features/edit-settings/model/useSystemSettings'
import { AppClientError } from '../../../../src/shared/api/tauri/errors'
import type { SystemClient } from '../../../../src/entities/system'

const client: SystemClient = {
	getSettings: vi.fn().mockResolvedValue({
		version: '0.2.4',
		autostartEnabled: false,
		shortcut: { available: true, label: 'Win+Shift+Q', error: null },
		scanSettings: {
			autoScanFixedDrives: true,
			includedPaths: [],
			excludedPaths: [],
		},
		fixedDrives: ['C:\\'],
	}),
	setAutostart: vi.fn(),
	setScanSettings: vi.fn(),
	getUninstallHistory: vi.fn().mockResolvedValue([]),
	clearUninstallHistory: vi.fn(),
	pickFolder: vi.fn(),
	openTelegram: vi.fn(),
	openGithub: vi.fn(),
	openAppsSettings: vi.fn(),
}

describe('useSystemSettings', () => {
	it('does not expose settings failure internals', async () => {
		const { result } = renderHook(() =>
			useSystemSettings({
				client: {
					...client,
					getSettings: vi
						.fn()
						.mockRejectedValue(new Error('C:\\Users\\Example\\private-settings-detail')),
				},
			}),
		)

		await waitFor(() =>
			expect(result.current.error).toBe(
				'The operation could not be completed. Try again.',
			),
		)
	})

	it('allows only one catalog maintenance operation at a time', async () => {
		let finishForce: (() => void) | undefined
		const force = vi.fn(
			() =>
				new Promise<void>(resolve => {
					finishForce = resolve
				}),
		)
		const reset = vi.fn().mockResolvedValue(undefined)
		const { result } = renderHook(() =>
			useSystemSettings({
				client,
				onForceFullScan: force,
				onResetCatalogCache: reset,
			}),
		)

		let forcing: Promise<void>
		let resetting: Promise<void>
		act(() => {
			forcing = result.current.forceFullScan()
			resetting = result.current.resetCatalogCache()
		})
		expect(force).toHaveBeenCalledOnce()
		expect(reset).not.toHaveBeenCalled()
		finishForce?.()
		await act(async () => Promise.all([forcing, resetting]))
	})

	it.each(['force', 'reset'] as const)(
		'does not expose %s scan cancellation as a settings error',
		async operation => {
			const cancellation = new AppClientError(
				'SCAN_CANCELLED',
				'Application scan cancelled.',
			)
			const { result } = renderHook(() =>
				useSystemSettings({
					client,
					onForceFullScan: vi.fn().mockRejectedValue(cancellation),
					onResetCatalogCache: vi
						.fn()
						.mockRejectedValue(cancellation),
				}),
			)

			await act(() =>
				operation === 'force'
					? result.current.forceFullScan()
					: result.current.resetCatalogCache(),
			)

			expect(result.current.error).toBeNull()
		},
	)
})
