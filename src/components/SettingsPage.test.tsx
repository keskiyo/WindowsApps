import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { SystemClient } from '../types'
import { SettingsPage } from './SettingsPage'

describe('SettingsPage', () => {
	it('loads system settings and toggles Windows startup', async () => {
		const client: SystemClient = {
			getSettings: vi.fn().mockResolvedValue({ version: '0.1.0', autostartEnabled: false, shortcut: { available: true, label: 'Win+Shift+Q', error: null } }),
			setAutostart: vi.fn().mockResolvedValue(undefined),
			openTelegram: vi.fn().mockResolvedValue(undefined),
		}
		render(<SettingsPage client={client} />)
		expect(await screen.findByText('Version 0.1.0')).toBeInTheDocument()
		expect(screen.getByText('Win+Shift+Q')).toBeInTheDocument()
		await userEvent.click(screen.getByRole('switch', { name: 'Launch when Windows starts' }))
		expect(client.setAutostart).toHaveBeenCalledWith(true)
		await userEvent.click(screen.getByRole('button', { name: 'Open @keskiyo on Telegram' }))
		expect(client.openTelegram).toHaveBeenCalledOnce()
	})
})
