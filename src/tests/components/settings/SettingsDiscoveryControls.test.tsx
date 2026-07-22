import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SettingsDiscoveryControls } from '../../../components/settings/SettingsDiscoveryControls'
import type { SystemSettings } from '../../../types'

const settings: SystemSettings = {
	version: '0.2.6',
	autostartEnabled: false,
	shortcut: { available: true, label: 'Win+Shift+Q', error: null },
	scanSettings: {
		autoScanFixedDrives: true,
		includedPaths: [],
		excludedPaths: [],
	},
	fixedDrives: ['C:\\'],
}

describe('SettingsDiscoveryControls', () => {
	it('saves a selected scan folder through the provided settings actions', async () => {
		const onAddPath = vi.fn()
		const onPickFolder = vi.fn().mockResolvedValue(String.raw`F:\Tools`)
		render(
			<SettingsDiscoveryControls
				settings={settings}
				saving={false}
				onSaveScanSettings={vi.fn()}
				onAddPath={onAddPath}
				onRemovePath={vi.fn()}
				onPickFolder={onPickFolder}
			/>,
		)

		await userEvent.click(
			screen.getByRole('button', { name: 'Browse for scan folder' }),
		)

		expect(onPickFolder).toHaveBeenCalledOnce()
		expect(onAddPath).toHaveBeenCalledWith('includedPaths', String.raw`F:\Tools`)
	})
})
