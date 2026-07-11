import { act, renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useUpdater } from '../../hooks/useUpdater'

const check = vi.fn()

vi.mock('@tauri-apps/plugin-updater', () => ({
	check: () => check(),
}))

vi.mock('@tauri-apps/plugin-process', () => ({
	relaunch: vi.fn(),
}))

function update(version: string) {
	return {
		version,
		body: '## Highlights\n- Test update.',
		downloadAndInstall: vi.fn(),
	}
}

describe('useUpdater', () => {
	beforeEach(() => {
		check.mockReset()
		localStorage.clear()
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
	})
})
