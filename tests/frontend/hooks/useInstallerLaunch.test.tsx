import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useInstallerLaunch } from '../../../src/hooks/useInstallerLaunch'
import type { AppInfo } from '../../../src/types'

function app(artifactKind?: AppInfo['artifactKind']): AppInfo {
	return {
		id: artifactKind ?? 'application',
		name: artifactKind ?? 'Application',
		path: 'C:\\app.exe',
		iconBase64: null,
		artifactKind,
		category: artifactKind ? 'installers_docs' : 'other',
		launchKind: 'executable',
		sourceKind: 'portable',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
	}
}

describe('useInstallerLaunch', () => {
	it('launches applications and docs immediately but waits for installer confirmation', async () => {
		const launch = vi.fn().mockResolvedValue(undefined)
		const { result } = renderHook(() => useInstallerLaunch(launch))

		await act(() => result.current.requestLaunch(app()))
		await act(() => result.current.requestLaunch(app('documentation')))
		await act(() => result.current.requestLaunch(app('installer')))

		expect(launch).toHaveBeenCalledTimes(2)
		expect(result.current.app?.artifactKind).toBe('installer')
	})

	it('cancels without launching and confirms a pending installer only once', async () => {
		let resolveLaunch: () => void = () => undefined
		const launch = vi.fn(
			() =>
				new Promise<void>(resolve => {
					resolveLaunch = () => resolve()
				}),
		)
		const { result } = renderHook(() => useInstallerLaunch(launch))
		await act(() => result.current.requestLaunch(app('installer')))
		act(() => result.current.cancel())
		expect(launch).not.toHaveBeenCalled()

		await act(() => result.current.requestLaunch(app('installer')))
		let first!: Promise<void>
		act(() => {
			first = result.current.confirm()
			void result.current.confirm()
		})
		expect(launch).toHaveBeenCalledOnce()
		expect(result.current.pending).toBe(true)
		resolveLaunch()
		await act(() => first)
		expect(result.current.app).toBeNull()
		expect(result.current.pending).toBe(false)
	})
})
