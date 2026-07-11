import type { UninstallMechanism } from './app'

export interface GlobalShortcutStatus {
	available: boolean
	label: string
	error: string | null
}

export interface SystemSettings {
	version: string
	autostartEnabled: boolean
	shortcut: GlobalShortcutStatus
	scanSettings: ScanSettings
	fixedDrives: string[]
}

export interface ScanSettings {
	autoScanFixedDrives: boolean
	includedPaths: string[]
	excludedPaths: string[]
}

export interface UninstallHistoryEntry {
	id: string
	timestamp: number
	appName: string
	publisher: string | null
	mechanism: UninstallMechanism
	result: 'succeeded' | 'failed'
}

export interface SystemClient {
	getSettings(): Promise<SystemSettings>
	setAutostart(enabled: boolean): Promise<void>
	setScanSettings(settings: ScanSettings): Promise<ScanSettings>
	getUninstallHistory(): Promise<UninstallHistoryEntry[]>
	clearUninstallHistory(): Promise<void>
	pickFolder(): Promise<string | null>
	openTelegram(): Promise<void>
	openGithub(): Promise<void>
	openRelease?(version: string): Promise<void>
	/** Non-null when this process is an outdated leftover copy of the app. */
	staleCopyStatus?(): Promise<StaleCopyInfo | null>
	/** Launch the registered (newer) installed copy; the backend exits this one. */
	openInstalledCopy?(): Promise<void>
}

export interface StaleCopyInfo {
	installedVersion: string
	installLocation: string
}
