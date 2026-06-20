import { invoke } from '@tauri-apps/api/core'
import type { SystemClient } from '../types'

export const tauriSystemClient: SystemClient = {
	getSettings: () => invoke('get_system_settings'),
	setAutostart: enabled => invoke('set_autostart', { enabled }),
	openTelegram: () => invoke('open_telegram'),
}
