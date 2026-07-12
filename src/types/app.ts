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
export type AppVisibilityClass = 'primary' | 'auxiliary' | 'rejected'
export type AppVisibilityReason =
	| 'start_menu_registration'
	| 'windows_app_registration'
	| 'steam_registration'
	| 'portable_candidate'
	| 'product_metadata'
	| 'registered_product'
	| 'executable_product_match'
	| 'runtime_directory'
	| 'product_component'
	| 'documentation_shortcut'
	| 'installer'
	| 'maintenance_executable'
	| 'insufficient_launch_evidence'

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
	productName?: string | null
	originalFilename?: string | null
	installLocation: string | null
	canUninstall: boolean
	launchArguments?: string | null
	canonicalIdentity?: string | null
	userPromoted?: boolean
	visibilityClass?: AppVisibilityClass
	visibilityScore?: number
	visibilityReasons?: AppVisibilityReason[]
}

export type AppView = 'all' | 'favorites' | 'settings' | 'hidden' | 'auxiliary'

export interface CatalogSnapshot {
	apps: AppInfo[]
	hasCache: boolean
	generation?: number
	diagnostics?: CatalogDiagnostics | null
}

export interface CatalogDiagnostics {
	completedAt: number
	durationMs: number
	mode: 'watch' | 'startup' | 'refresh' | 'force'
	totalApps: number
	sourceCounts: Record<string, number>
	visibilityCounts?: Record<string, number>
	added: number
	removed: number
	updated: number
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
	productName?: string
	originalFilename?: string
	installLocation?: string
	canUninstall?: boolean
}

export interface ScanProgress {
	stage: string
	location: string | null
	completedRoots: number
	totalRoots: number
}

// Best-effort launch outcome from the backend: 'ready' when the launched process reached
// its input-idle state, 'failed' when the shell/process reported an error. Absent for
// launches where no process handle is available (Store/UWP, shell hand-off) — the UI's
// ceiling timer covers those.
export interface LaunchStatus {
	id: string
	state: 'ready' | 'failed'
}

export interface AppsClient {
	getApps(): Promise<CatalogSnapshot>
	refreshApps(): Promise<AppInfo[]>
	forceFullScan?(): Promise<AppInfo[]>
	resetCatalogCache?(): Promise<AppInfo[]>
	clearIconCache?(): Promise<void>
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
	onScanProgress(
		handler: (progress: ScanProgress) => void,
	): Promise<() => void>
	onLaunchStatus?(
		handler: (status: LaunchStatus) => void,
	): Promise<() => void>
}
