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
	error: string | null
	activeView: AppView
	favoriteAppIds: string[]
	categoryOrder: AppCategory[]
	collapsedCategories: AppCategory[]
	categoryOverrides: Record<string, AppCategory>
	hiddenAppIds: string[]
	categories: CategoryDefinition[]
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

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error)
}

export function createAppStore(
	client: AppsClient,
	storage: Storage = globalThis.localStorage,
	idFactory: () => string = () => `custom:${crypto.randomUUID()}`,
): StoreApi<AppState> {
	const preferences = readPreferences(storage)
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
			error: null,
			activeView: 'all',
			favoriteAppIds: preferences.favoriteAppIds,
			categoryOrder: preferences.categoryOrder,
			collapsedCategories: preferences.collapsedCategories,
			categoryOverrides: preferences.categoryOverrides,
			hiddenAppIds: preferences.hiddenAppIds,
			categories: preferences.categories,
			async load() {
				set({ isLoading: true, error: null })
				try {
					const snapshot = await client.getApps()
					set({
						apps: snapshot.apps,
						hasCache: snapshot.hasCache,
						catalogGeneration: snapshot.generation ?? 0,
					})
				} catch (error) {
					set({ error: errorMessage(error) })
				} finally {
					set({ isLoading: false })
				}
			},
			async initialize() {
				const disposers: Array<() => void> = []
				const subscribe = async <T>(
					registration:
						| ((handler: (value: T) => void) => Promise<() => void>)
						| undefined,
					handler: (value: T) => void,
				) => {
					if (registration)
						disposers.push(await registration(handler))
				}
				await subscribe(client.onCatalogDelta, get().applyDelta)
				await subscribe(client.onCatalogPatches, get().applyPatches)
				await subscribe(client.onCatalogChanged, summary =>
					set({ catalogChange: summary }),
				)
				disposers.push(await client.onAppsUpdated(get().replaceApps))
				disposers.push(
					await client.onScanProgress(scanProgress =>
						set({ scanProgress }),
					),
				)
				await get().load()
				if (get().hasCache) await client.startBackgroundSync?.()
				return () => disposers.forEach(dispose => dispose())
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
			async cancelScan() {
				await client.cancelScan()
			},
			async launch(app) {
				set({ error: null })
				try {
					await client.launchApp({
						launchKind: app.launchKind,
						path: app.path,
					})
				} catch (error) {
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
				set({ apps })
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
					for (const app of delta.upserted) apps.set(app.id, app)
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

export function selectFilteredApps(state: AppState): AppInfo[] {
	const apps = selectVisibleApps(state)
	const query = state.query.trim().toLocaleLowerCase()
	return query
		? apps.filter(app =>
				[app.name, app.publisher, app.description].some(value =>
					value?.toLocaleLowerCase().includes(query),
				),
			)
		: apps
}

export function selectVisibleApps(state: AppState): AppInfo[] {
	const apps = selectCategorizedApps(state)
	if (state.activeView === 'settings') return []
	if (state.activeView === 'hidden')
		return apps.filter(app => state.hiddenAppIds.includes(app.id))
	const visible = apps.filter(app => !state.hiddenAppIds.includes(app.id))
	return state.activeView === 'favorites'
		? visible.filter(app => state.favoriteAppIds.includes(app.id))
		: visible
}

export function selectCategorizedApps(state: AppState): AppInfo[] {
	return deduplicateVisibleApps(
		state.apps.map(app => ({
			...app,
			category: state.categoryOverrides[app.id] ?? app.category,
		})),
	)
}

export const appStore = createAppStore(tauriAppsClient)

function deduplicateVisibleApps(apps: AppInfo[]): AppInfo[] {
	const unique: AppInfo[] = []
	for (const app of apps) {
		const index = unique.findIndex(existing =>
			isVisibleDuplicate(existing, app),
		)
		if (index === -1) {
			unique.push(app)
			continue
		}
		if (visibleCandidateScore(app) > visibleCandidateScore(unique[index])) {
			unique[index] = mergeVisibleApp(app, unique[index])
		} else {
			unique[index] = mergeVisibleApp(unique[index], app)
		}
	}
	return unique
}

function isVisibleDuplicate(left: AppInfo, right: AppInfo): boolean {
	if (
		left.id === right.id ||
		left.path.toLowerCase() === right.path.toLowerCase()
	)
		return true
	const leftName = normalizedVisibleFamily(left.name)
	const rightName = normalizedVisibleFamily(right.name)
	if (leftName !== rightName) {
		return (
			!publishersConflict(left, right) &&
			stripLauncherSuffix(leftName) === stripLauncherSuffix(rightName) &&
			(left.launchKind === 'shortcut' ||
				right.launchKind === 'shortcut' ||
				samePathFamily(left, right))
		)
	}
	return !publishersConflict(left, right) || (oneIsShortcut(left, right) && samePathFamily(left, right))
}

function mergeVisibleApp(primary: AppInfo, secondary: AppInfo): AppInfo {
	return {
		...primary,
		iconBase64: primary.iconBase64 ?? secondary.iconBase64,
		description: primary.description ?? secondary.description,
		version: primary.version ?? secondary.version,
		publisher: primary.publisher ?? secondary.publisher,
		installLocation: primary.installLocation ?? secondary.installLocation,
		canUninstall: primary.canUninstall || secondary.canUninstall,
	}
}

function visibleCandidateScore(app: AppInfo): number {
	if (app.sourceKind === 'steam') return 5
	if (app.launchKind === 'shortcut') return 4
	if (app.launchKind === 'executable') return 3
	if (app.launchKind === 'app_user_model_id') return 2
	return 0
}

function oneIsShortcut(left: AppInfo, right: AppInfo): boolean {
	return left.launchKind === 'shortcut' || right.launchKind === 'shortcut'
}

function publishersConflict(left: AppInfo, right: AppInfo): boolean {
	const leftPublisher = normalizedPublisher(left.publisher)
	const rightPublisher = normalizedPublisher(right.publisher)
	return Boolean(
		leftPublisher && rightPublisher && leftPublisher !== rightPublisher,
	)
}

function normalizedPublisher(value: string | null): string {
	return (
		value
			?.toLocaleLowerCase()
			.replace(/\b(incorporated|inc|llc|ltd|limited|corp|corporation)\b/g, '')
			.replace(/[^a-z0-9а-яё]+/giu, '')
			.trim() ?? ''
	)
}

function samePathFamily(left: AppInfo, right: AppInfo): boolean {
	const leftPath = left.path.toLocaleLowerCase()
	const rightPath = right.path.toLocaleLowerCase()
	const leftFamily = normalizedVisibleFamily(left.name)
	const rightFamily = normalizedVisibleFamily(right.name)
	return (
		leftPath.includes(leftFamily) ||
		leftPath.includes(rightFamily) ||
		rightPath.includes(leftFamily) ||
		rightPath.includes(rightFamily)
	)
}

function normalizedVisibleFamily(name: string): string {
	let value = name.toLocaleLowerCase().split(/\s+/).join(' ').trim()
	for (const suffix of [
		' (64bit)',
		' (32bit)',
		' (64-bit)',
		' (32-bit)',
		' x64',
		' x86',
	]) {
		if (value.endsWith(suffix)) value = value.slice(0, -suffix.length)
	}
	value = stripVersionSuffix(value)
	return value
}

function stripVersionSuffix(value: string): string {
	const match = value.match(/^(.*?)[\s_-]+v?\d+(?:[._-]\d+){1,3}$/i)
	return match?.[1]?.trim() || value
}

function stripLauncherSuffix(value: string): string {
	return value.endsWith(' launcher')
		? value.slice(0, -' launcher'.length).trim()
		: value
}
