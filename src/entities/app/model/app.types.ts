import type { AppCategory } from '../../category'

export type AppLaunchKind = 'executable' | 'shortcut' | 'app_user_model_id'
export type AppSourceKind =
	'registry' | 'start_menu' | 'start_apps' | 'msix' | 'steam' | 'portable'
export type UninstallMechanism = 'registered_command' | 'msi' | 'msix'
export type AppVisibilityClass = 'primary' | 'auxiliary' | 'rejected'
export type AppArtifactKind = 'application' | 'installer' | 'documentation'
export type AppArchitecture =
	'x86' | 'x64' | 'arm64' | 'notApplicable' | 'unknown'
export type AppSignatureStatus = 'verified' | 'unsigned' | 'unavailable'
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
	| 'framework_package'
	| 'shell_location_shortcut'
	| 'sdk_sample'
	| 'command_environment'
	| 'console_application'
	| 'insufficient_launch_evidence'
	| 'unknown'

export interface AppInfo {
	id: string
	name: string
	path: string
	iconBase64: string | null
	artifactKind?: AppArtifactKind
	category: AppCategory
	launchKind: AppLaunchKind
	sourceKind: AppSourceKind
	description: string | null
	version: string | null
	publisher: string | null
	productName?: string | null
	originalFilename?: string | null
	installLocation: string | null
	resolvedPath?: string | null
	canUninstall: boolean
	launchArguments?: string | null
	canonicalIdentity?: string | null
	preferenceIdentity?: string | null
	userPromoted?: boolean
	userInstaller?: boolean
	visibilityClass?: AppVisibilityClass
	visibilityScore?: number
	visibilityReasons?: AppVisibilityReason[]
}

export interface AppDetails {
	fileSizeBytes: number | null
	fileCreatedAt: number | null
	fileModifiedAt: number | null
	architecture: AppArchitecture
	signature: AppSignatureStatus
	executableExists: boolean | null
	installLocationExists: boolean | null
	canOpenFolder?: boolean
}

export type AppView =
	| 'all'
	| 'favorites'
	| 'more'
	| 'scenarios'
	| 'settings'
	| 'hidden'
	| 'auxiliary'
	| 'installers_docs'

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

export interface LaunchStatus {
	id: string
	state: 'ready' | 'failed'
}

export interface CloseAppsResult {
	closed: number
	notRunning: number
	unavailable: number
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
	closeApps(ids: string[]): Promise<CloseAppsResult>
	getAppDetails(id: string): Promise<AppDetails>
	openAppFolder(id: string): Promise<void>
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
