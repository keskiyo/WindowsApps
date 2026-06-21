import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SettingsPage } from '../../../components/settings/SettingsPage'
import type { SystemClient } from '../../../types'

describe('SettingsPage', () => {
	it('loads system settings and toggles Windows startup', async () => {
		const client: SystemClient = {
			getSettings: vi
				.fn()
				.mockResolvedValue({
					version: '0.1.0',
					autostartEnabled: false,
					shortcut: {
						available: true,
						label: 'Win+Shift+Q',
						error: null,
					},
					scanSettings: {
						autoScanFixedDrives: true,
						includedPaths: [String.raw`D:\Games`],
						excludedPaths: [],
					},
					fixedDrives: ['C:\\', 'D:\\', 'E:\\'],
				}),
			setAutostart: vi.fn().mockResolvedValue(undefined),
			setScanSettings: vi.fn().mockImplementation(async settings => settings),
			pickFolder: vi.fn().mockResolvedValue(String.raw`F:\Stick\Tools`),
			openTelegram: vi.fn().mockResolvedValue(undefined),
		}
		render(<SettingsPage client={client} />)
		expect(await screen.findByText('Version 0.1.0')).toBeInTheDocument()
		expect(screen.getByText('Win+Shift+Q')).toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('switch', { name: 'Launch when Windows starts' }),
		)
		expect(client.setAutostart).toHaveBeenCalledWith(true)
		await userEvent.click(
			screen.getByRole('button', { name: 'Open @keskiyo on Telegram' }),
		)
		expect(client.openTelegram).toHaveBeenCalledOnce()
		expect(screen.getByText('Fixed local drives')).toBeInTheDocument()
		expect(screen.getByText('E:\\')).toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('button', { name: 'Browse for scan folder' }),
		)
		expect(client.setScanSettings).toHaveBeenCalledWith({
			autoScanFixedDrives: true,
			includedPaths: [String.raw`D:\Games`, String.raw`F:\Stick\Tools`],
			excludedPaths: [],
		})
	})

	it('adds a removable-drive folder picked from the native dialog', async () => {
		const client: SystemClient = {
			getSettings: vi.fn().mockResolvedValue({
				version: '0.1.0',
				autostartEnabled: false,
				shortcut: { available: true, label: 'Win+Shift+Q', error: null },
				scanSettings: {
					autoScanFixedDrives: true,
					includedPaths: [],
					excludedPaths: [],
				},
				fixedDrives: ['C:\\'],
			}),
			setAutostart: vi.fn().mockResolvedValue(undefined),
			setScanSettings: vi.fn().mockImplementation(async settings => settings),
			pickFolder: vi.fn().mockResolvedValue(String.raw`F:\Stick\Tools`),
			openTelegram: vi.fn().mockResolvedValue(undefined),
		}
		render(<SettingsPage client={client} />)
		await screen.findByText('Version 0.1.0')
		await userEvent.click(
			screen.getByRole('button', { name: 'Browse for scan folder' }),
		)
		expect(client.pickFolder).toHaveBeenCalledOnce()
		expect(client.setScanSettings).toHaveBeenCalledWith({
			autoScanFixedDrives: true,
			includedPaths: [String.raw`F:\Stick\Tools`],
			excludedPaths: [],
		})
	})
})
