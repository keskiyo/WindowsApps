import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SettingsPage } from '../../../components/settings/SettingsPage'
import type { SystemClient } from '../../../types'

describe('SettingsPage', () => {
	const settings = {
		version: '0.1.0',
		autostartEnabled: false,
		shortcut: { available: true, label: 'Win+Shift+Q', error: null },
		scanSettings: {
			autoScanFixedDrives: true,
			includedPaths: [],
			excludedPaths: [],
		},
		fixedDrives: ['C:\\'],
	}

	const systemClient = (): SystemClient => ({
		getSettings: vi.fn().mockResolvedValue(settings),
		setAutostart: vi.fn().mockResolvedValue(undefined),
		setScanSettings: vi.fn().mockImplementation(async value => value),
		getUninstallHistory: vi.fn().mockResolvedValue([]),
		clearUninstallHistory: vi.fn().mockResolvedValue(undefined),
		pickFolder: vi.fn().mockResolvedValue(null),
		openTelegram: vi.fn().mockResolvedValue(undefined),
	})

	it('confirms and starts a forced full scan', async () => {
		const onForceFullScan = vi.fn().mockResolvedValue(undefined)
		render(
			<SettingsPage
				client={systemClient()}
				onForceFullScan={onForceFullScan}
			/>,
		)
		await screen.findByText('Version 0.1.0')

		await userEvent.click(
			screen.getByRole('button', { name: 'Force full scan' }),
		)
		expect(onForceFullScan).not.toHaveBeenCalled()
		await userEvent.click(
			screen.getByRole('button', { name: 'Confirm full scan' }),
		)

		expect(onForceFullScan).toHaveBeenCalledOnce()
	})

	it('confirms and resets the catalog cache', async () => {
		const onResetCatalogCache = vi.fn().mockResolvedValue(undefined)
		render(
			<SettingsPage
				client={systemClient()}
				onForceFullScan={vi.fn().mockResolvedValue(undefined)}
				onResetCatalogCache={onResetCatalogCache}
			/>,
		)
		await screen.findByText('Version 0.1.0')

		await userEvent.click(
			screen.getByRole('button', { name: 'Reset catalog cache' }),
		)
		expect(onResetCatalogCache).not.toHaveBeenCalled()
		await userEvent.click(
			screen.getByRole('button', { name: 'Confirm reset' }),
		)

		expect(onResetCatalogCache).toHaveBeenCalledOnce()
	})

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
			getUninstallHistory: vi.fn().mockResolvedValue([
				{
					id: 'history-1',
					timestamp: 1_800_000_000,
					appName: 'Visual Studio Code',
					publisher: 'Microsoft',
					mechanism: 'registered_command',
					result: 'succeeded',
				},
			]),
			clearUninstallHistory: vi.fn().mockResolvedValue(undefined),
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
		expect(screen.getByText('Visual Studio Code')).toBeInTheDocument()
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
			getUninstallHistory: vi.fn().mockResolvedValue([]),
			clearUninstallHistory: vi.fn().mockResolvedValue(undefined),
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

	it('clears uninstall history only after confirmation', async () => {
		const client = systemClient()
		vi.mocked(client.getUninstallHistory).mockResolvedValue([
			{
				id: 'history-1',
				timestamp: 1_800_000_000,
				appName: 'Visual Studio Code',
				publisher: 'Microsoft',
				mechanism: 'registered_command',
				result: 'succeeded',
			},
		])
		render(<SettingsPage client={client} />)
		expect(await screen.findByText('Visual Studio Code')).toBeInTheDocument()

		await userEvent.click(screen.getByRole('button', { name: 'Clear' }))
		expect(client.clearUninstallHistory).not.toHaveBeenCalled()
		await userEvent.click(
			screen.getByRole('button', { name: 'Confirm clear' }),
		)

		expect(client.clearUninstallHistory).toHaveBeenCalledOnce()
		expect(screen.getByText('No uninstall history yet.')).toBeInTheDocument()
	})
})
