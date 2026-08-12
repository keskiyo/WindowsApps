import type { StoreApi } from 'zustand/vanilla'
import type { AppCategory, CategoryDefinition } from '../../entities/category'
import type { Scenario, ScenarioList, ScenarioRunRecord } from '../../entities/scenario'
import type { LegacyCanonicalPreferences } from './preferences'
import type { PreferenceTransferResult } from './preferences'
import type {
	AppHydrationPatch,
	AppInfo,
	AppView,
	CatalogChangeSummary,
	CatalogDelta,
	CatalogDiagnostics,
	CloseAppsResult,
	ScanProgress,
	UninstallPreview,
} from '../../entities/app'

export interface AppState {
	apps: AppInfo[]
	query: string
	isLoading: boolean
	isRefreshing: boolean
	scanProgress: ScanProgress | null
	hasCache: boolean
	catalogGeneration: number
	catalogChange: CatalogChangeSummary | null
	catalogDiagnostics: CatalogDiagnostics | null
	error: string | null
	activeView: AppView
	favoriteAppIds: string[]
	favoriteAppIdentities: string[]
	categoryOrder: AppCategory[]
	collapsedCategories: AppCategory[]
	categoryOverrides: Record<string, AppCategory>
	categoryOverrideIdentities: Record<string, AppCategory>
	hiddenAppIds: string[]
	hiddenAppIdentities: string[]
	promotedAppIds: string[]
	promotedAppIdentities: string[]
	installerAppIds: string[]
	installerAppIdentities: string[]
	scenarios: Scenario[]
	favoriteScenarioIds: string[]
	scenarioHistory: ScenarioRunRecord[]
	firstSeenAt: Record<string, number>
	legacyCanonicalPreferences: LegacyCanonicalPreferences
	unknownPreferenceFields: Record<string, unknown>
	preferencesPersisted: boolean
	categories: CategoryDefinition[]
	launchingIds: string[]
	markLaunching(id: string): void
	clearLaunching(id: string): void
	createCategory(
		label: string,
	): { ok: true; id: string } | { ok: false; error: string }
	renameCategory(
		id: string,
		label: string,
	): { ok: true } | { ok: false; error: string }
	deleteCategory(id: string): { ok: true } | { ok: false; error: string }
	load(): Promise<void>
	initialize(): Promise<() => void>
	refresh(): Promise<void>
	forceFullScan(): Promise<void>
	resetCatalogCache(): Promise<void>
	clearIconCache(): Promise<void>
	repairMissingIcons(): Promise<void>
	hydrateVisibleIcons(ids: string[]): Promise<void>
	cancelScan(): Promise<void>
	launch(app: AppInfo): Promise<void>
	closeApps(ids: string[]): Promise<CloseAppsResult>
	createScenario(
		name: string,
	): { ok: true; id: string } | { ok: false; error: string }
	renameScenario(
		id: string,
		name: string,
	): { ok: true } | { ok: false; error: string }
	deleteScenario(id: string): void
	addScenarioApp(
		id: string,
		list: ScenarioList,
		identity: string,
	): { ok: true } | { ok: false; error: string }
	removeScenarioApp(id: string, list: ScenarioList, identity: string): void
	toggleFavoriteScenario(id: string): void
	recordScenarioRun(record: ScenarioRunRecord): void
	getUninstallPreview(id: string): Promise<UninstallPreview>
	uninstall(id: string): Promise<void>
	setQuery(query: string): void
	setActiveView(view: AppView): void
	exportPreferences(): string
	validatePreferencesImport(source: string): PreferenceTransferResult
	importPreferences(source: string): PreferenceTransferResult
	restorePreferencesBackup(): PreferenceTransferResult
	toggleFavorite(id: string): void
	hideApp(id: string): void
	restoreApp(id: string): void
	promoteAuxiliary(id: string): void
	demoteAuxiliary(id: string): void
	reorderCategory(active: AppCategory, over: AppCategory): void
	moveApp(id: string, category: AppCategory): void
	toggleCategory(category: AppCategory): void
	replaceApps(apps: AppInfo[]): void
	applyDelta(delta: CatalogDelta): void
	applyPatches(patches: AppHydrationPatch[]): void
	clearCatalogChange(): void
	subscribe(): Promise<() => void>
	subscribeScanProgress(): Promise<() => void>
}

export type SetAppState = StoreApi<AppState>['setState']
export type GetAppState = StoreApi<AppState>['getState']

export type PersistPreferences = () => void
