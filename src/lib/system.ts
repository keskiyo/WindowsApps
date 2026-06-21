import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { SystemClient } from '../types'

export const tauriSystemClient: SystemClient = {
	getSettings: () => invoke('get_system_settings'),
	setAutostart: enabled => invoke('set_autostart', { enabled }),
	setScanSettings: settings => invoke('set_scan_settings', { settings }),
	pickFolder: () =>
		open({ directory: true }).then(result =>
			typeof result === 'string' ? result : null,
		),
	openTelegram: () => invoke('open_telegram'),
}
