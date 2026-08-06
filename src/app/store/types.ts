import type { StoreApi } from 'zustand/vanilla'
import type { AppCategory, CategoryDefinition } from '../../entities/category'
import type { Scenario, ScenarioList } from '../../entities/scenario'
import type { LegacyCanonicalPreferences } from './preferences'
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
	// `*AppIds` are the runtime projection the components read (`.includes(app.id)`); they are
	// re-derived from `*AppIdentities` against the current catalog on load, so favorites and
	// hidden survive an id change from a dedup rule change. `*AppIdentities` is the durable form.
	favoriteAppIds: string[]
	favoriteAppIdentities: string[]
	categoryOrder: AppCategory[]
	collapsedCategories: AppCategory[]
	// `categoryOverrides` is the runtime id-keyed projection; `categoryOverrideIdentities` is the
	// durable form (keyed by `canonicalIdentity`) that survives a Force full scan, Reset cache, or
	// dedup rule change. The selector resolves identity-first so the override follows the app.
	categoryOverrides: Record<string, AppCategory>
	categoryOverrideIdentities: Record<string, AppCategory>
	hiddenAppIds: string[]
	hiddenAppIdentities: string[]
	promotedAppIds: string[]
	promotedAppIdentities: string[]
	// Applications the user filed into Installers & Docs by hand. Same id/identity pairing as the
	// sets above; the selector turns a mark into `artifactKind: 'installer'`.
	installerAppIds: string[]
	installerAppIdentities: string[]
	/** Named launch/close lists, keyed by card identity so they survive a rescan. */
	scenarios: Scenario[]
	/** When each card was first seen in the catalog; the only source of "recently added". */
	firstSeenAt: Record<string, number>
	legacyCanonicalPreferences: LegacyCanonicalPreferences
	/** False once a preferences write was refused (quota, private mode, storage disabled). */
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
	/** Closes a whole batch in one request; see `CloseAppsResult` for what the counts mean. */
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
	getUninstallPreview(id: string): Promise<UninstallPreview>
	uninstall(id: string): Promise<void>
	setQuery(query: string): void
	setActiveView(view: AppView): void
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

/**
 * What every action module receives. The store is created once by `createAppStore`; each module
 * contributes a slice of actions built against these, never against a store it reaches for itself.
 */
export type SetAppState = StoreApi<AppState>['setState']
export type GetAppState = StoreApi<AppState>['getState']

/** Writes the current state to storage and reports a refused write through `preferencesPersisted`. */
export type PersistPreferences = () => void
