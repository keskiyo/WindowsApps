import { open } from '@tauri-apps/plugin-dialog'
import type { SystemClient } from '../types'
import { invokeTauri } from './tauri'

export const tauriSystemClient: SystemClient = {
	getSettings: () => invokeTauri('get_system_settings'),
	setAutostart: enabled => invokeTauri('set_autostart', { enabled }),
	setScanSettings: settings => invokeTauri('set_scan_settings', { settings }),
	getUninstallHistory: () => invokeTauri('get_uninstall_history'),
	clearUninstallHistory: () => invokeTauri('clear_uninstall_history'),
	pickFolder: () =>
		open({ directory: true }).then(result =>
			typeof result === 'string' ? result : null,
		),
	openTelegram: () => invokeTauri('open_telegram'),
	openGithub: () => invokeTauri('open_github'),
	openRelease: version => invokeTauri('open_release', { version }),
	staleCopyStatus: () => invokeTauri('stale_copy_status'),
	openInstalledCopy: () => invokeTauri('open_installed_copy'),
}
