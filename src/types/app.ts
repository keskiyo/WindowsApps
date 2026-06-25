import type { AppCategory } from './category'

export type AppLaunchKind = 'executable' | 'shortcut' | 'app_user_model_id'
export type AppSourceKind =
	| 'registry'
	| 'start_menu'
	| 'start_apps'
	| 'msix'
	| 'steam'
	| 'portable'
export type UninstallMechanism = 'registered_command' | 'msi' | 'msix'

export interface AppInfo {
	id: string
	name: string
	path: string
	iconBase64: string | null
	category: AppCategory
	launchKind: AppLaunchKind
	sourceKind: AppSourceKind
	description: string | null
	version: string | null
	publisher: string | null
	installLocation: string | null
	canUninstall: boolean
}

export type AppView = 'all' | 'favorites' | 'settings' | 'hidden' | 'vpn'

export interface VpnInfo {
	id: string
	name: string
	installed: boolean
	connected: boolean
}

export interface VpnClient {
	list(): Promise<VpnInfo[]>
	set(id: string, enabled: boolean): Promise<VpnInfo>
	setup(id: string): Promise<VpnInfo>
}

export interface CatalogSnapshot {
	apps: AppInfo[]
	hasCache: boolean
	generation?: number
}

export interface UninstallPreview {
	appName: string
	publisher: string | null
	source: AppSourceKind
	mechanism: UninstallMechanism
	command: string
}

export interface CatalogChangeSummary {
	added: number
	removed: number
	updated: number
}

export interface CatalogDelta {
	generation: number
	upserted: AppInfo[]
	removedIds: string[]
	summary: CatalogChangeSummary
}

export interface AppHydrationPatch {
	id: string
	generation: number
	iconBase64?: string
	description?: string
	version?: string
	publisher?: string
	installLocation?: string
	canUninstall?: boolean
}

export interface ScanProgress {
	stage: string
	location: string | null
	completedRoots: number
	totalRoots: number
}

export interface AppsClient {
	getApps(): Promise<CatalogSnapshot>
	refreshApps(): Promise<AppInfo[]>
	forceFullScan?(): Promise<AppInfo[]>
	resetCatalogCache?(): Promise<AppInfo[]>
	hydrateVisibleIcons?(ids: string[]): Promise<void>
	startBackgroundSync?(): Promise<void>
	cancelScan(): Promise<void>
	launchApp(app: Pick<AppInfo, 'id'>): Promise<void>
	getUninstallPreview(id: string): Promise<UninstallPreview>
	uninstallApp(id: string): Promise<void>
	onAppsUpdated(handler: (apps: AppInfo[]) => void): Promise<() => void>
	onCatalogDelta?(handler: (delta: CatalogDelta) => void): Promise<() => void>
	onCatalogPatches?(
		handler: (patches: AppHydrationPatch[]) => void,
	): Promise<() => void>
	onCatalogChanged?(
		handler: (summary: CatalogChangeSummary) => void,
	): Promise<() => void>
	onScanProgress(handler: (progress: ScanProgress) => void): Promise<() => void>
}
