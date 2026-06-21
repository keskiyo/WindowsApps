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

export interface SystemClient {
	getSettings(): Promise<SystemSettings>
	setAutostart(enabled: boolean): Promise<void>
	setScanSettings(settings: ScanSettings): Promise<ScanSettings>
	pickFolder(): Promise<string | null>
	openTelegram(): Promise<void>
}
