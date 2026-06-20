export interface GlobalShortcutStatus {
	available: boolean
	label: string
	error: string | null
}

export interface SystemSettings {
	version: string
	autostartEnabled: boolean
	shortcut: GlobalShortcutStatus
}

export interface SystemClient {
	getSettings(): Promise<SystemSettings>
	setAutostart(enabled: boolean): Promise<void>
	openTelegram(): Promise<void>
}
