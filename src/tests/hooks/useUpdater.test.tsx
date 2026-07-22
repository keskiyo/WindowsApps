import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useUpdater } from '../../hooks/useUpdater'

const check = vi.fn()
const relaunch = vi.fn()

vi.mock('@tauri-apps/plugin-updater', () => ({
	check: () => check(),
}))

vi.mock('@tauri-apps/plugin-process', () => ({
	relaunch: () => relaunch(),
}))

function update(version: string) {
	return {
		version,
		date: '2026-07-11T10:00:00Z',
		body: '## Highlights\n- Test update.',
		rawJson: {
			packageSize: 5_600_000,
			releaseUrl: `https://github.com/keskiyo/WindowsApps/releases/tag/v${version}`,
		},
		download: vi.fn(),
		install: vi.fn(),
	}
}

describe('useUpdater', () => {
	beforeEach(() => {
		check.mockReset()
		relaunch.mockReset()
		localStorage.clear()
	})

	afterEach(() => {
		vi.restoreAllMocks()
	})

	it('does not auto-show an update version dismissed earlier', async () => {
		check.mockResolvedValue(update('0.2.2'))

		const { result, unmount } = renderHook(() => useUpdater())

		await waitFor(() =>
			expect(result.current.update?.version).toBe('0.2.2'),
		)
		act(() => result.current.dismiss())
		expect(result.current.update).toBeNull()
		unmount()

		const second = renderHook(() => useUpdater())

		await waitFor(() => expect(check).toHaveBeenCalledTimes(2))
		expect(second.result.current.update).toBeNull()
		expect(localStorage.getItem('windows-apps.dismissed-update-version')).toBe(
			'0.2.2',
		)
	})

	it('manual checks show a dismissed version again', async () => {
		localStorage.setItem('windows-apps.dismissed-update-version', '0.2.2')
		check.mockResolvedValue(update('0.2.2'))

		const { result } = renderHook(() => useUpdater({ autoCheck: false }))

		await act(async () => {
			await result.current.checkNow()
		})

		expect(result.current.update?.version).toBe('0.2.2')
		expect(result.current.status).toBe('available')
		expect(result.current.update).toMatchObject({
			version: '0.2.2',
			date: '2026-07-11T10:00:00Z',
			packageSize: 5_600_000,
			releaseUrl:
				'https://github.com/keskiyo/WindowsApps/releases/tag/v0.2.2',
		})
	})

	it('coalesces an automatic and manual check while preserving manual visibility', async () => {
		localStorage.setItem('windows-apps.dismissed-update-version', '0.2.2')
		let resolveCheck: ((value: ReturnType<typeof update>) => void) | undefined
		check.mockImplementation(
			() =>
				new Promise(resolve => {
					resolveCheck = resolve
				}),
		)
		const { result } = renderHook(() => useUpdater())
		await waitFor(() => expect(check).toHaveBeenCalledOnce())

		let manualCheck: Promise<void>
		act(() => {
			manualCheck = result.current.checkNow()
		})
		resolveCheck?.(update('0.2.2'))
		await act(async () => manualCheck)

		expect(check).toHaveBeenCalledOnce()
		expect(result.current.update?.version).toBe('0.2.2')
	})

	it('shows a useful download error and allows retrying the same update', async () => {
		const download = vi
			.fn()
			.mockRejectedValueOnce(
				new Error('Download request failed with status: 404 Not Found'),
			)
			.mockResolvedValueOnce(undefined)
		const install = vi.fn().mockResolvedValue(undefined)
		check.mockResolvedValue({
			...update('0.2.3'),
			download,
			install,
		})

		const { result } = renderHook(() => useUpdater({ autoCheck: false }))
		await act(async () => result.current.checkNow())
		await act(async () => result.current.install())

		expect(result.current.phase).toBe('failed')
		expect(result.current.error).toBe(
			'The update package is unavailable. Try again later or download it from GitHub.',
		)

		await act(async () => result.current.install())

		expect(download).toHaveBeenCalledTimes(2)
		expect(install).toHaveBeenCalledOnce()
		expect(relaunch).toHaveBeenCalledOnce()
	})

	it('shows a permission-specific error when quiet install cannot write files', async () => {
		const download = vi.fn().mockResolvedValue(undefined)
		const install = vi
			.fn()
			.mockRejectedValue(new Error('Access is denied. Permission denied'))
		check.mockResolvedValue({
			...update('0.2.4'),
			download,
			install,
		})

		const { result } = renderHook(() => useUpdater({ autoCheck: false }))
		await act(async () => result.current.checkNow())
		await act(async () => result.current.install())

		expect(result.current.phase).toBe('failed')
		expect(result.current.error).toBe(
			'The update could not write the new version. Reinstall Windows Apps for the current user or download the installer manually.',
		)
		expect(relaunch).not.toHaveBeenCalled()
	})

	it('does not log raw updater failures to the browser console', async () => {
		const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined)
		check.mockResolvedValue({
			...update('0.2.4'),
			download: vi
				.fn()
				.mockRejectedValue(new Error('C:\\Users\\Maks\\private-updater-detail')),
		})

		const { result } = renderHook(() => useUpdater({ autoCheck: false }))
		await act(async () => result.current.checkNow())
		await act(async () => result.current.install())

		expect(consoleError).not.toHaveBeenCalled()
	})

	it('tracks downloaded bytes and uses separate download and install stages', async () => {
		const download = vi.fn(async callback => {
			callback({ event: 'Started', data: { contentLength: 1000 } })
			callback({ event: 'Progress', data: { chunkLength: 250 } })
		})
		let resolveInstall: (() => void) | undefined
		const install = vi.fn(
			() =>
				new Promise<void>(resolve => {
					resolveInstall = resolve
				}),
		)
		check.mockResolvedValue({ ...update('0.2.3'), download, install })
		const { result } = renderHook(() => useUpdater({ autoCheck: false }))
		await act(async () => result.current.checkNow())

		let installing: Promise<void>
		act(() => {
			installing = result.current.install()
		})
		await waitFor(() => expect(result.current.phase).toBe('installing'))
		expect(result.current.downloadedBytes).toBe(250)
		expect(result.current.totalBytes).toBe(1000)
		expect(result.current.progress).toBe(100)
		expect(download).toHaveBeenCalledOnce()
		expect(install).toHaveBeenCalledOnce()

		await act(async () => {
			resolveInstall?.()
			await installing
		})
		expect(result.current.phase).toBe('restarting')
		expect(relaunch).toHaveBeenCalledOnce()
	})

	it('does not start the same installation twice before React rerenders', async () => {
		let resolveDownload: (() => void) | undefined
		const download = vi.fn(
			() =>
				new Promise<void>(resolve => {
					resolveDownload = resolve
				}),
		)
		const install = vi.fn().mockResolvedValue(undefined)
		check.mockResolvedValue({ ...update('0.2.3'), download, install })
		const { result } = renderHook(() => useUpdater({ autoCheck: false }))
		await act(async () => result.current.checkNow())

		let first: Promise<void>
		let second: Promise<void>
		act(() => {
			first = result.current.install()
			second = result.current.install()
		})
		expect(download).toHaveBeenCalledOnce()
		resolveDownload?.()
		await act(async () => Promise.all([first, second]))
		expect(install).toHaveBeenCalledOnce()
	})
})
