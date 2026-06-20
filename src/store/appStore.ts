import { createStore, type StoreApi } from 'zustand/vanilla'
import { readPreferences, writePreferences } from '../lib/preferences'
import { tauriAppsClient } from '../lib/tauri'
import type {
	AppCategory,
	AppInfo,
	AppsClient,
	AppView,
	CategoryDefinition,
	ScanProgress,
} from '../types'

export interface AppState {
	apps: AppInfo[]
	query: string
	isLoading: boolean
	isRefreshing: boolean
	scanProgress: ScanProgress | null
	hasCache: boolean
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
	refresh(): Promise<void>
	cancelScan(): Promise<void>
	launch(app: AppInfo): Promise<void>
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
					set({ apps: snapshot.apps, hasCache: snapshot.hasCache })
				} catch (error) {
					set({ error: errorMessage(error) })
				} finally {
					set({ isLoading: false })
				}
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
			subscribe() {
				return client.onAppsUpdated(apps => set({ apps }))
			},
			subscribeScanProgress() {
				return client.onScanProgress(scanProgress => set({ scanProgress }))
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
	return state.apps.map(app => ({
		...app,
		category: state.categoryOverrides[app.id] ?? app.category,
	}))
}

export const appStore = createAppStore(tauriAppsClient)
