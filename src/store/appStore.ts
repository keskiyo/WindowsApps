import { createStore, type StoreApi } from 'zustand/vanilla'
import { chooseCustomCategoryAccent } from '../lib/categoryAccents'
import { toAppClientError } from '../lib/clientError'
import {
	readPreferences,
	writePreferences,
	type LegacyCanonicalPreferences,
} from '../lib/preferences'
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

// Ceiling for the "launching" visual when the backend can't report real readiness
// (Store/UWP, shell hand-off). Cleared early by the launch://status event when available.
const LAUNCH_CEILING_MS = 12000
const HYDRATION_BATCH_SIZE = 128

function errorMessage(error: unknown): string | null {
	const clientError = toAppClientError(error)
	return clientError.code === 'SCAN_CANCELLED' ? null : clientError.message
}

function identityOf(app: AppInfo): string {
	return app.preferenceIdentity ?? app.canonicalIdentity ?? app.id
}

function addUnique(list: string[], value: string): string[] {
	return list.includes(value) ? list : [...list, value]
}

/**
 * Reconcile a favorite/hidden set against a freshly loaded catalog. Legacy ids (saved before
 * identities existed, or by an older app version) are resolved to their identity, then the
 * runtime id list is re-derived from the identities against the current apps — so a selection
 * follows the app across a dedup rule change that renamed its id.
 */
function reconcileSelection(
	apps: AppInfo[],
	ids: string[],
	identities: string[],
	legacyCanonicalIdentities: string[],
): { ids: string[]; identities: string[]; unresolvedLegacy: string[] } {
	const byId = new Map(apps.map(app => [app.id, app]))
	const mergedIdentities = new Set(identities)
	const unresolvedLegacy = new Set(legacyCanonicalIdentities)
	for (const legacyId of ids) {
		const app = byId.get(legacyId)
		if (!app) continue
		mergedIdentities.add(identityOf(app))
		if (app.canonicalIdentity)
			unresolvedLegacy.delete(app.canonicalIdentity)
	}
	const byCanonicalIdentity = new Map<string, AppInfo[]>()
	for (const app of apps) {
		if (!app.canonicalIdentity) continue
		const group = byCanonicalIdentity.get(app.canonicalIdentity) ?? []
		group.push(app)
		byCanonicalIdentity.set(app.canonicalIdentity, group)
	}
	for (const legacyIdentity of unresolvedLegacy) {
		const matches = byCanonicalIdentity.get(legacyIdentity)
		if (matches?.length !== 1) continue
		mergedIdentities.add(identityOf(matches[0]))
		unresolvedLegacy.delete(legacyIdentity)
	}
	const currentIds = apps
		.filter(app => mergedIdentities.has(identityOf(app)))
		.map(app => app.id)
	return {
		ids: currentIds,
		identities: [...mergedIdentities],
		unresolvedLegacy: [...unresolvedLegacy],
	}
}

/**
 * Reconcile the manual category overrides against a freshly loaded catalog, mirroring
 * `reconcileSelection`. Legacy id-keyed entries (written before identities existed) are folded into
 * the identity map for apps present in the catalog, then the runtime id map is re-derived from the
 * identity map — so an override follows its app across a dedup rule change that renamed its id.
 */
function reconcileOverrides(
	apps: AppInfo[],
	idOverrides: Record<string, AppCategory>,
	identityOverrides: Record<string, AppCategory>,
	legacyCanonicalOverrides: Record<string, AppCategory>,
): {
	overrides: Record<string, AppCategory>
	overrideIdentities: Record<string, AppCategory>
	unresolvedLegacy: Record<string, AppCategory>
} {
	const byId = new Map(apps.map(app => [app.id, app]))
	const mergedIdentities: Record<string, AppCategory> = { ...identityOverrides }
	const unresolvedLegacy = { ...legacyCanonicalOverrides }
	for (const [legacyId, category] of Object.entries(idOverrides)) {
		const app = byId.get(legacyId)
		if (app && !(identityOf(app) in mergedIdentities))
			mergedIdentities[identityOf(app)] = category
		if (app?.canonicalIdentity)
			delete unresolvedLegacy[app.canonicalIdentity]
	}
	const byCanonicalIdentity = new Map<string, AppInfo[]>()
	for (const app of apps) {
		if (!app.canonicalIdentity) continue
		const group = byCanonicalIdentity.get(app.canonicalIdentity) ?? []
		group.push(app)
		byCanonicalIdentity.set(app.canonicalIdentity, group)
	}
	for (const [legacyIdentity, category] of Object.entries(
		unresolvedLegacy,
	)) {
		const matches = byCanonicalIdentity.get(legacyIdentity)
		if (matches?.length !== 1) continue
		if (!(identityOf(matches[0]) in mergedIdentities))
			mergedIdentities[identityOf(matches[0])] = category
		delete unresolvedLegacy[legacyIdentity]
	}
	const overrides: Record<string, AppCategory> = {}
	for (const app of apps) {
		const category = mergedIdentities[identityOf(app)]
		if (category) overrides[app.id] = category
	}
	return {
		overrides,
		overrideIdentities: mergedIdentities,
		unresolvedLegacy,
	}
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

	async function hydrateIds(ids: string[]): Promise<void> {
		if (!client.hydrateVisibleIcons) return
		for (let start = 0; start < ids.length; start += HYDRATION_BATCH_SIZE) {
			try {
				await client.hydrateVisibleIcons(
					ids.slice(start, start + HYDRATION_BATCH_SIZE),
				)
			} catch {
				// Hydration is best-effort. One failed batch must not starve later
				// visible cards; background hydration can retry the failed IDs.
			}
		}
	}

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
			const persisted = writePreferences(storage, {
				version: 8,
				categories: state.categories,
				categoryOrder: state.categoryOrder,
				favoriteAppIds: state.favoriteAppIds,
				favoriteAppIdentities: state.favoriteAppIdentities,
				collapsedCategories: state.collapsedCategories,
				categoryOverrides: state.categoryOverrides,
				categoryOverrideIdentities: state.categoryOverrideIdentities,
				hiddenAppIds: state.hiddenAppIds,
				hiddenAppIdentities: state.hiddenAppIdentities,
				promotedAppIds: state.promotedAppIds,
				promotedAppIdentities: state.promotedAppIdentities,
				legacyCanonicalPreferences:
					state.legacyCanonicalPreferences,
				unknownFields: preferences.unknownFields,
			})
			// A refused write leaves the UI showing changes that will not survive a restart,
			// so the condition is surfaced instead of being swallowed.
			if (persisted !== state.preferencesPersisted) {
				set({ preferencesPersisted: persisted })
			}
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
			favoriteAppIdentities: preferences.favoriteAppIdentities,
			categoryOrder: preferences.categoryOrder,
			collapsedCategories: preferences.collapsedCategories,
			categoryOverrides: preferences.categoryOverrides,
			categoryOverrideIdentities: preferences.categoryOverrideIdentities,
			hiddenAppIds: preferences.hiddenAppIds,
			hiddenAppIdentities: preferences.hiddenAppIdentities,
			promotedAppIds: preferences.promotedAppIds,
			promotedAppIdentities: preferences.promotedAppIdentities,
			legacyCanonicalPreferences:
				preferences.legacyCanonicalPreferences,
			preferencesPersisted: true,
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
					const legacy = get().legacyCanonicalPreferences
					const favorites = reconcileSelection(
						snapshot.apps,
						get().favoriteAppIds,
						get().favoriteAppIdentities,
						legacy.favorite,
					)
					const hidden = reconcileSelection(
						snapshot.apps,
						get().hiddenAppIds,
						get().hiddenAppIdentities,
						legacy.hidden,
					)
					const promoted = reconcileSelection(
						snapshot.apps,
						get().promotedAppIds,
						get().promotedAppIdentities,
						legacy.promoted,
					)
					const overrides = reconcileOverrides(
						snapshot.apps,
						get().categoryOverrides,
						get().categoryOverrideIdentities,
						legacy.categoryOverrides,
					)
					set({
						apps: snapshot.apps,
						hasCache: snapshot.hasCache,
						catalogGeneration: snapshot.generation ?? 0,
						catalogDiagnostics: snapshot.diagnostics ?? null,
						promotedAppIds: promoted.ids,
						promotedAppIdentities: promoted.identities,
						favoriteAppIds: favorites.ids,
						favoriteAppIdentities: favorites.identities,
						hiddenAppIds: hidden.ids,
						hiddenAppIdentities: hidden.identities,
						categoryOverrides: overrides.overrides,
						categoryOverrideIdentities: overrides.overrideIdentities,
						legacyCanonicalPreferences: {
							favorite: favorites.unresolvedLegacy,
							hidden: hidden.unresolvedLegacy,
							promoted: promoted.unresolvedLegacy,
							categoryOverrides: overrides.unresolvedLegacy,
						},
					})
					persist()
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
						// Registration is all-or-nothing. Each listener was awaited in turn, so a
						// rejection partway through left the earlier ones attached with no owner
						// to detach them, and the rejected promise was cached — every later
						// initialize() returned that same failure without ever retrying. On
						// failure the accumulated disposers run and the cached state is cleared,
						// so a transient bridge error is recoverable.
						try {
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
						} catch (error) {
							disposers.splice(0).forEach(dispose => {
								try {
									dispose()
								} catch {
									// One listener that refuses to detach must not strand the rest.
								}
							})
							initializationUsers = Math.max(
								0,
								initializationUsers - 1,
							)
							initializationDispose = null
							initializationPromise = null
							throw error
						}
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
				} finally {
					set({ isRefreshing: false, scanProgress: null })
				}
			},
			async hydrateVisibleIcons(ids) {
				if (!ids.length || !client.hydrateVisibleIcons) return
				await hydrateIds(ids)
			},
			async clearIconCache() {
				if (!client.clearIconCache) return
				await client.clearIconCache()
				await get().hydrateVisibleIcons(
					get().apps.map(app => app.id),
				)
			},
			async repairMissingIcons() {
				await get().hydrateVisibleIcons(
					get()
						.apps.filter(app => !app.iconBase64)
						.map(app => app.id),
				)
			},
			async cancelScan() {
				await client.cancelScan()
			},
			// Actions reject; they do not also write `error`. `error` is the *background*
			// failure channel — work nobody is waiting on, which only `load()` produces — and
			// App toasts it. An action already has an owner for its message (`useAppFeedback`
			// for launch/refresh/uninstall, `useSystemSettings` for the maintenance scans), so
			// writing it here too reported one failure twice, the second time with no Retry.
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
					throw error
				}
			},
			async getUninstallPreview(id) {
				return client.getUninstallPreview(id)
			},
			async uninstall(id) {
				set({ error: null })
				return client.uninstallApp(id)
			},
			setQuery(query) {
				set({ query })
			},
			setActiveView(activeView) {
				set({ activeView })
			},
			toggleFavorite(id) {
				set(state => {
					const app = state.apps.find(item => item.id === id)
					const promoted = app
						? state.promotedAppIds.includes(app.id) ||
							state.promotedAppIdentities.includes(
								identityOf(app),
							)
						: false
					if (app?.visibilityClass === 'auxiliary' && !promoted)
						return state
					const identity = app ? identityOf(app) : id
					const wasFavorite = state.favoriteAppIds.includes(id)
					return {
						favoriteAppIds: wasFavorite
							? state.favoriteAppIds.filter(appId => appId !== id)
							: [...state.favoriteAppIds, id],
						favoriteAppIdentities: wasFavorite
							? state.favoriteAppIdentities.filter(
									item => item !== identity,
								)
							: addUnique(state.favoriteAppIdentities, identity),
					}
				})
				persist()
			},
			hideApp(id) {
				set(state => {
					if (state.hiddenAppIds.includes(id)) return state
					const app = state.apps.find(item => item.id === id)
					const identity = app ? identityOf(app) : id
					return {
						hiddenAppIds: [...state.hiddenAppIds, id],
						hiddenAppIdentities: addUnique(
							state.hiddenAppIdentities,
							identity,
						),
					}
				})
				persist()
			},
			restoreApp(id) {
				set(state => {
					const app = state.apps.find(item => item.id === id)
					const identity = app ? identityOf(app) : id
					return {
						hiddenAppIds: state.hiddenAppIds.filter(
							appId => appId !== id,
						),
						hiddenAppIdentities: state.hiddenAppIdentities.filter(
							item => item !== identity,
						),
					}
				})
				persist()
			},
			promoteAuxiliary(id) {
				const app = get().apps.find(item => item.id === id)
				const identity = app ? identityOf(app) : id
				set(state => ({
					promotedAppIdentities: state.promotedAppIdentities.includes(
						identity,
					)
						? state.promotedAppIdentities
						: [...state.promotedAppIdentities, identity],
				}))
				persist()
			},
			demoteAuxiliary(id) {
				const app = get().apps.find(item => item.id === id)
				const identity = app ? identityOf(app) : id
				set(state => ({
					promotedAppIds: state.promotedAppIds.filter(
						appId => appId !== id,
					),
					promotedAppIdentities: state.promotedAppIdentities.filter(
						item => item !== identity,
					),
					favoriteAppIds: state.favoriteAppIds.filter(
						appId => appId !== id,
					),
					favoriteAppIdentities: state.favoriteAppIdentities.filter(
						item => item !== identity,
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
				set(state => {
					const app = state.apps.find(item => item.id === id)
					const identity = app ? identityOf(app) : id
					return {
						categoryOverrides: {
							...state.categoryOverrides,
							[id]: category,
						},
						categoryOverrideIdentities: {
							...state.categoryOverrideIdentities,
							[identity]: category,
						},
					}
				})
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
				const accent = chooseCustomCategoryAccent(
					get().categories.flatMap(category =>
						category.builtIn || !category.accent
							? []
							: [category.accent],
					),
				)
				set(state => ({
					categories: [
						...state.categories,
						{ id, label: value, builtIn: false, accent },
					],
					categoryOrder: [id, ...state.categoryOrder],
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
					categoryOverrideIdentities: Object.fromEntries(
						Object.entries(state.categoryOverrideIdentities).map(
							([identity, category]) => [
								identity,
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
					return {
						apps: apps.map(app =>
							mergeIcon(previous.get(app.id), app),
						),
					}
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

// Pure selectors/filters live in ./selectors. Re-exported here so existing imports from
// '../store/appStore' keep working after the split.
export {
	filterAppsByQuery,
	filterVisibleApps,
	rankAppsByQuery,
	rankAppsByQueryTop,
	selectCatalogCounts,
	selectCategorizedApps,
	selectFilteredApps,
	selectVisibleApps,
} from './selectors'
