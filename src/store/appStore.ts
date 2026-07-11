import { createStore, type StoreApi } from 'zustand/vanilla'
import { readPreferences, writePreferences } from '../lib/preferences'
import { tauriAppsClient } from '../lib/tauri'
import type {
	AppCategory,
	AppHydrationPatch,
	AppInfo,
	AppsClient,
	AppView,
	CatalogChangeSummary,
	CatalogDelta,
	CatalogDiagnostics,
	CategoryDefinition,
	ScanProgress,
	UninstallPreview,
} from '../types'

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
	categoryOrder: AppCategory[]
	collapsedCategories: AppCategory[]
	categoryOverrides: Record<string, AppCategory>
	hiddenAppIds: string[]
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
	getUninstallPreview(id: string): Promise<UninstallPreview>
	uninstall(id: string): Promise<void>
	setQuery(query: string): void
	setActiveView(view: AppView): void
	toggleFavorite(id: string): void
	hideApp(id: string): void
	restoreApp(id: string): void
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

// Ceiling for the "launching" visual when the backend can't report real readiness
// (Store/UWP, shell hand-off). Cleared early by the launch://status event when available.
const LAUNCH_CEILING_MS = 12000

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error)
}

// Preserve an already-loaded icon when an incoming app record has none, so background
// syncs (which ship icon-less app data) don't blank the grid before patches re-arrive.
function mergeIcon(previous: AppInfo | undefined, next: AppInfo): AppInfo {
	return previous?.iconBase64 && !next.iconBase64
		? { ...next, iconBase64: previous.iconBase64 }
		: next
}

export function createAppStore(
	client: AppsClient,
	storage: Storage = globalThis.localStorage,
	idFactory: () => string = () => `custom:${crypto.randomUUID()}`,
): StoreApi<AppState> {
	const preferences = readPreferences(storage)
	const launchTimers = new Map<string, ReturnType<typeof setTimeout>>()
	let initializationPromise: Promise<() => void> | null = null
	let initializationDispose: (() => void) | null = null
	let initializationUsers = 0

	function releaseInitialization() {
		initializationUsers = Math.max(0, initializationUsers - 1)
		if (initializationUsers > 0) return
		initializationDispose?.()
		initializationDispose = null
		initializationPromise = null
	}

	return createStore<AppState>((set, get) => {
		function persist() {
			const state = get()
			writePreferences(storage, {
				version: 4,
				categories: state.categories,
				categoryOrder: state.categoryOrder,
				favoriteAppIds: state.favoriteAppIds,
				collapsedCategories: state.collapsedCategories,
				categoryOverrides: state.categoryOverrides,
				hiddenAppIds: state.hiddenAppIds,
			})
		}

		return {
			apps: [],
			query: '',
			isLoading: true,
			isRefreshing: false,
			scanProgress: null,
			hasCache: false,
			catalogGeneration: 0,
			catalogChange: null,
			catalogDiagnostics: null,
			error: null,
			activeView: 'all',
			favoriteAppIds: preferences.favoriteAppIds,
			categoryOrder: preferences.categoryOrder,
			collapsedCategories: preferences.collapsedCategories,
			categoryOverrides: preferences.categoryOverrides,
			hiddenAppIds: preferences.hiddenAppIds,
			categories: preferences.categories,
			launchingIds: [],
			markLaunching(id) {
				set(state =>
					state.launchingIds.includes(id)
						? state
						: { launchingIds: [...state.launchingIds, id] },
				)
			},
			clearLaunching(id) {
				const timer = launchTimers.get(id)
				if (timer) {
					clearTimeout(timer)
					launchTimers.delete(id)
				}
				set(state =>
					state.launchingIds.includes(id)
						? {
								launchingIds: state.launchingIds.filter(
									appId => appId !== id,
								),
							}
						: state,
				)
			},
			async load() {
				set({ isLoading: true, error: null })
				try {
					const snapshot = await client.getApps()
					set({
						apps: snapshot.apps,
						hasCache: snapshot.hasCache,
						catalogGeneration: snapshot.generation ?? 0,
						catalogDiagnostics: snapshot.diagnostics ?? null,
					})
				} catch (error) {
					set({ error: errorMessage(error) })
				} finally {
					set({ isLoading: false })
				}
			},
			async initialize() {
				initializationUsers += 1
				if (!initializationPromise) {
					initializationPromise = (async () => {
						const disposers: Array<() => void> = []
						const subscribe = async <T>(
							registration:
								| ((
										handler: (value: T) => void,
									) => Promise<() => void>)
								| undefined,
							handler: (value: T) => void,
						) => {
							if (registration)
								disposers.push(await registration(handler))
						}
						await subscribe(client.onCatalogDelta, get().applyDelta)
						await subscribe(
							client.onCatalogPatches,
							get().applyPatches,
						)
						await subscribe(client.onCatalogChanged, summary =>
							set({ catalogChange: summary }),
						)
						disposers.push(
							await client.onAppsUpdated(get().replaceApps),
						)
						disposers.push(
							await client.onScanProgress(scanProgress =>
								set({ scanProgress }),
							),
						)
						await subscribe(client.onLaunchStatus, status =>
							get().clearLaunching(status.id),
						)
						await get().load()
						if (get().hasCache) await client.startBackgroundSync?.()
						initializationDispose = () =>
							disposers.splice(0).forEach(dispose => dispose())
						return releaseInitialization
					})()
				}
				return initializationPromise
			},
			async refresh() {
				set({ isRefreshing: true, error: null })
				try {
					set({ apps: await client.refreshApps(), hasCache: true })
				} catch (error) {
					set({ error: errorMessage(error) })
					throw error
				} finally {
					set({ isRefreshing: false, scanProgress: null })
				}
			},
			async forceFullScan() {
				set({ isRefreshing: true, error: null })
				try {
					const apps = client.forceFullScan
						? await client.forceFullScan()
						: await client.refreshApps()
					set({ apps, hasCache: true })
				} catch (error) {
					set({ error: errorMessage(error) })
					throw error
				} finally {
					set({ isRefreshing: false, scanProgress: null })
				}
			},
			async resetCatalogCache() {
				set({ isRefreshing: true, error: null, apps: [] })
				try {
					const apps = client.resetCatalogCache
						? await client.resetCatalogCache()
						: await get()
								.forceFullScan()
								.then(() => get().apps)
					set({ apps, hasCache: true })
				} catch (error) {
					set({ error: errorMessage(error) })
					throw error
				} finally {
					set({ isRefreshing: false, scanProgress: null })
				}
			},
			async hydrateVisibleIcons(ids) {
				if (!ids.length || !client.hydrateVisibleIcons) return
				try {
					await client.hydrateVisibleIcons(ids)
				} catch {
					// Icon hydration is an optimization path; the normal background
					// hydrator still runs and app launching must not be affected.
				}
			},
			async clearIconCache() {
				if (!client.clearIconCache) return
				await client.clearIconCache()
				await client.hydrateVisibleIcons?.(get().apps.map(app => app.id))
			},
			async repairMissingIcons() {
				await client.hydrateVisibleIcons?.(
					get()
						.apps.filter(app => !app.iconBase64)
						.map(app => app.id),
				)
			},
			async cancelScan() {
				await client.cancelScan()
			},
			async launch(app) {
				set({ error: null })
				get().markLaunching(app.id)
				const existing = launchTimers.get(app.id)
				if (existing) clearTimeout(existing)
				launchTimers.set(
					app.id,
					setTimeout(
						() => get().clearLaunching(app.id),
						LAUNCH_CEILING_MS,
					),
				)
				try {
					await client.launchApp({ id: app.id })
				} catch (error) {
					get().clearLaunching(app.id)
					set({ error: errorMessage(error) })
					throw error
				}
			},
			async getUninstallPreview(id) {
				return client.getUninstallPreview(id)
			},
			async uninstall(id) {
				set({ error: null })
				try {
					return await client.uninstallApp(id)
				} catch (error) {
					set({ error: errorMessage(error) })
					throw error
				}
			},
			setQuery(query) {
				set({ query })
			},
			setActiveView(activeView) {
				set({ activeView })
			},
			toggleFavorite(id) {
				set(state => ({
					favoriteAppIds: state.favoriteAppIds.includes(id)
						? state.favoriteAppIds.filter(appId => appId !== id)
						: [...state.favoriteAppIds, id],
				}))
				persist()
			},
			hideApp(id) {
				set(state => ({
					hiddenAppIds: state.hiddenAppIds.includes(id)
						? state.hiddenAppIds
						: [...state.hiddenAppIds, id],
				}))
				persist()
			},
			restoreApp(id) {
				set(state => ({
					hiddenAppIds: state.hiddenAppIds.filter(
						appId => appId !== id,
					),
				}))
				persist()
			},
			reorderCategory(active, over) {
				set(state => {
					const from = state.categoryOrder.indexOf(active)
					const to = state.categoryOrder.indexOf(over)
					if (from < 0 || to < 0 || from === to) return state
					const categoryOrder = [...state.categoryOrder]
					categoryOrder.splice(
						to,
						0,
						categoryOrder.splice(from, 1)[0],
					)
					return { categoryOrder }
				})
				persist()
			},
			moveApp(id, category) {
				set(state => ({
					categoryOverrides: {
						...state.categoryOverrides,
						[id]: category,
					},
				}))
				persist()
			},
			createCategory(label) {
				const value = label.trim()
				if (!value) return { ok: false, error: 'Enter a category name' }
				if (
					get().categories.some(
						category =>
							category.label.toLocaleLowerCase() ===
							value.toLocaleLowerCase(),
					)
				)
					return { ok: false, error: 'Category name already exists' }
				const id = idFactory()
				set(state => ({
					categories: [
						...state.categories,
						{ id, label: value, builtIn: false },
					],
					categoryOrder: [...state.categoryOrder, id],
				}))
				persist()
				return { ok: true, id }
			},
			renameCategory(id, label) {
				const value = label.trim()
				if (!value) return { ok: false, error: 'Enter a category name' }
				if (
					get().categories.some(
						category =>
							category.id !== id &&
							category.label.toLocaleLowerCase() ===
								value.toLocaleLowerCase(),
					)
				)
					return { ok: false, error: 'Category name already exists' }
				if (!get().categories.some(category => category.id === id))
					return { ok: false, error: 'Category not found' }
				set(state => ({
					categories: state.categories.map(category =>
						category.id === id
							? { ...category, label: value }
							: category,
					),
				}))
				persist()
				return { ok: true }
			},
			deleteCategory(id) {
				const category = get().categories.find(
					category => category.id === id,
				)
				if (!category || category.builtIn)
					return {
						ok: false,
						error: 'Built-in categories cannot be deleted',
					}
				set(state => ({
					categories: state.categories.filter(
						category => category.id !== id,
					),
					categoryOrder: state.categoryOrder.filter(
						category => category !== id,
					),
					collapsedCategories: state.collapsedCategories.filter(
						category => category !== id,
					),
					categoryOverrides: Object.fromEntries(
						Object.entries(state.categoryOverrides).map(
							([appId, category]) => [
								appId,
								category === id ? 'other' : category,
							],
						),
					),
				}))
				persist()
				return { ok: true }
			},
			toggleCategory(category) {
				set(state => ({
					collapsedCategories: state.collapsedCategories.includes(
						category,
					)
						? state.collapsedCategories.filter(
								item => item !== category,
							)
						: [...state.collapsedCategories, category],
				}))
				persist()
			},
			replaceApps(apps) {
				set(state => {
					const previous = new Map(
						state.apps.map(app => [app.id, app]),
					)
					return { apps: apps.map(app => mergeIcon(previous.get(app.id), app)) }
				})
			},
			applyDelta(delta) {
				if (delta.generation < get().catalogGeneration) return
				set(state => {
					const removed = new Set(delta.removedIds)
					const apps = new Map(
						state.apps
							.filter(app => !removed.has(app.id))
							.map(app => [app.id, app]),
					)
					for (const app of delta.upserted)
						apps.set(app.id, mergeIcon(apps.get(app.id), app))
					return {
						apps: [...apps.values()],
						catalogGeneration: delta.generation,
					}
				})
			},
			applyPatches(patches) {
				const generation = get().catalogGeneration
				const current = patches.filter(
					patch => patch.generation === generation,
				)
				if (!current.length) return
				const byId = new Map(current.map(patch => [patch.id, patch]))
				set(state => ({
					apps: state.apps.map(app => {
						const patch = byId.get(app.id)
						return patch ? { ...app, ...patch, id: app.id } : app
					}),
				}))
			},
			clearCatalogChange() {
				set({ catalogChange: null })
			},
			subscribe() {
				return client.onAppsUpdated(apps => set({ apps }))
			},
			subscribeScanProgress() {
				return client.onScanProgress(scanProgress =>
					set({ scanProgress }),
				)
			},
		}
	})
}

export const appStore = createAppStore(tauriAppsClient)

// Pure selectors/filters live in ./selectors. Re-exported here so existing imports from
// '../store/appStore' keep working after the split.
export {
	filterVisibleApps,
	filterAppsByQuery,
	selectVisibleApps,
	selectFilteredApps,
	selectCategorizedApps,
} from './selectors'
