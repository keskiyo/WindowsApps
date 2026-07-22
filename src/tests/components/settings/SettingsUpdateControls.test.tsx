import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SettingsUpdateControls } from '../../../components/settings/SettingsUpdateControls'
import type { UpdaterState } from '../../../hooks/useUpdater'

function updater(overrides: Partial<UpdaterState> = {}): UpdaterState {
	return {
		update: null,
		installing: false,
		progress: null,
		downloadedBytes: 0,
		totalBytes: null,
		phase: 'idle',
		error: null,
		status: 'idle',
		checkNow: vi.fn().mockResolvedValue(undefined),
		install: vi.fn().mockResolvedValue(undefined),
		dismiss: vi.fn(),
		...overrides,
	}
}

describe('SettingsUpdateControls', () => {
	it('uses the shared updater and opens project links', async () => {
		const state = updater()
		const openGithub = vi.fn().mockResolvedValue(undefined)
		const openTelegram = vi.fn().mockResolvedValue(undefined)
		render(
			<SettingsUpdateControls
				updater={state}
				onOpenGithub={openGithub}
				onOpenTelegram={openTelegram}
			/>,
		)

		await userEvent.click(screen.getByRole('button', { name: 'Check updates' }))
		await userEvent.click(
			screen.getByRole('button', { name: 'Open Windows Apps on GitHub' }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Open @keskiyo on Telegram' }),
		)

		expect(state.checkNow).toHaveBeenCalledOnce()
		expect(openGithub).toHaveBeenCalledOnce()
		expect(openTelegram).toHaveBeenCalledOnce()
	})
})
