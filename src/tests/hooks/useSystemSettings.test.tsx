import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useSystemSettings } from '../../hooks/useSystemSettings'
import type { SystemClient } from '../../types'

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
}

describe('useSystemSettings', () => {
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
})
