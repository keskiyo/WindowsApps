import { describe, expect, it, vi } from 'vitest'
import { PREFERENCES_KEY } from '../../lib/preferences'
import {
	createAppStore,
	selectFilteredApps,
	selectVisibleApps,
} from '../../store/appStore'
import type { AppInfo, AppsClient } from '../../types'

function app(
	value: Partial<AppInfo> &
		Pick<AppInfo, 'id' | 'name' | 'path' | 'category'>,
): AppInfo {
	return {
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
		...value,
	}
}

const apps: AppInfo[] = [
	app({
		id: 'code',
		name: 'Visual Studio Code',
		path: 'C:\\Code.exe',
		category: 'development',
		description: 'Editor by Microsoft',
	}),
	app({
		id: 'chrome',
		name: 'Google Chrome',
		path: 'C:\\Chrome.exe',
		iconBase64: 'data:image/png;base64,abc',
		category: 'browsers',
		publisher: 'Google',
	}),
	app({
		id: 'codex',
		name: 'Codex',
		path: 'OpenAI.Codex!App',
		category: 'ai',
		launchKind: 'app_user_model_id',
		sourceKind: 'start_apps',
		publisher: 'OpenAI',
	}),
]

function client(overrides: Partial<AppsClient> = {}): AppsClient {
	return {
		getApps: vi.fn().mockResolvedValue({ apps, hasCache: true }),
		refreshApps: vi.fn().mockResolvedValue(apps.slice().reverse()),
		cancelScan: vi.fn().mockResolvedValue(undefined),
		launchApp: vi.fn().mockResolvedValue(undefined),
		uninstallApp: vi.fn().mockResolvedValue(undefined),
		onAppsUpdated: vi.fn().mockResolvedValue(() => undefined),
		onScanProgress: vi.fn().mockResolvedValue(() => undefined),
		...overrides,
	}
}

describe('app store', () => {
	it('loads applications and clears loading state', async () => {
		const store = createAppStore(client())
		await store.getState().load()
		expect(store.getState().apps).toEqual(apps)
		expect(store.getState().isLoading).toBe(false)
	})

	it('filters applications case-insensitively', () => {
		const store = createAppStore(client())
		store.setState({ apps, query: 'CHROME' })
		expect(selectFilteredApps(store.getState())).toEqual([apps[1]])
	})

	it('searches publisher and description', () => {
		const store = createAppStore(client())
		store.setState({ apps, query: 'openai' })
		expect(
			selectFilteredApps(store.getState()).map(item => item.id),
		).toEqual(['codex'])
		store.setState({ query: 'microsoft' })
		expect(
			selectFilteredApps(store.getState()).map(item => item.id),
		).toEqual(['code'])
	})

	it('replaces applications after refresh', async () => {
		const store = createAppStore(client())
		await store.getState().refresh()
		expect(store.getState().apps).toEqual(apps.slice().reverse())
		expect(store.getState().isRefreshing).toBe(false)
	})

	it('surfaces launch errors', async () => {
		const store = createAppStore(
			client({
				launchApp: vi
					.fn()
					.mockRejectedValue(new Error('Access denied')),
			}),
		)
		await expect(store.getState().launch(apps[0])).rejects.toThrow(
			'Access denied',
		)
		expect(store.getState().error).toBe('Access denied')
	})

	it('subscribes to background updates', async () => {
		let update: ((next: AppInfo[]) => void) | undefined
		const api = client({
			onAppsUpdated: vi.fn(async handler => {
				update = handler
				return () => undefined
			}),
		})
		const store = createAppStore(api)
		await store.getState().subscribe()
		update?.([apps[1]])
		expect(store.getState().apps).toEqual([apps[1]])
	})

	it('toggles favorites, persists them, and filters the favorites view', () => {
		const storage = {
			getItem: vi.fn(() => null),
			setItem: vi.fn(),
		} as unknown as Storage
		const store = createAppStore(client(), storage)
		store.setState({ apps })
		store.getState().toggleFavorite('code')
		store.getState().setActiveView('favorites')
		expect(selectVisibleApps(store.getState())).toEqual([apps[0]])
		expect(storage.setItem).toHaveBeenLastCalledWith(
			PREFERENCES_KEY,
			expect.stringContaining('"favoriteAppIds":["code"]'),
		)
	})

	it('hides and restores an app without losing its category or favorite', () => {
		const storage = {
			getItem: vi.fn(() => null),
			setItem: vi.fn(),
		} as unknown as Storage
		const store = createAppStore(client(), storage)
		store.setState({ apps })
		store.getState().toggleFavorite('code')
		store.getState().moveApp('code', 'ai')
		store.getState().hideApp('code')
		expect(
			selectVisibleApps(store.getState()).map(app => app.id),
		).not.toContain('code')
		store.getState().setActiveView('hidden')
		expect(selectVisibleApps(store.getState()).map(app => app.id)).toEqual([
			'code',
		])
		store.getState().restoreApp('code')
		expect(store.getState().categoryOverrides.code).toBe('ai')
		expect(store.getState().favoriteAppIds).toContain('code')
	})

	it('reorders categories and persists the order', () => {
		const storage = {
			getItem: vi.fn(() => null),
			setItem: vi.fn(),
		} as unknown as Storage
		const store = createAppStore(client(), storage)
		store.getState().reorderCategory('browsers', 'games')
		expect(store.getState().categoryOrder.slice(0, 2)).toEqual([
			'browsers',
			'games',
		])
		expect(storage.setItem).toHaveBeenCalled()
	})

	it('toggles collapsed categories through the preferences document', () => {
		const storage = {
			getItem: vi.fn(() => null),
			setItem: vi.fn(),
		} as unknown as Storage
		const store = createAppStore(client(), storage)
		store.getState().toggleCategory('development')
		expect(store.getState().collapsedCategories).toContain('development')
		expect(storage.setItem).toHaveBeenCalledWith(
			PREFERENCES_KEY,
			expect.any(String),
		)
	})

	it('applies and persists a manual category override', () => {
		const storage = {
			getItem: vi.fn(() => null),
			setItem: vi.fn(),
		} as unknown as Storage
		const store = createAppStore(client(), storage)
		store.setState({ apps })
		store.getState().moveApp('code', 'ai')
		expect(
			selectFilteredApps(store.getState()).find(
				item => item.id === 'code',
			)?.category,
		).toBe('ai')
		expect(storage.setItem).toHaveBeenLastCalledWith(
			PREFERENCES_KEY,
			expect.stringContaining('"code":"ai"'),
		)
	})

	it('creates, renames, and deletes a custom category while moving apps to Other', () => {
		const storage = {
			getItem: vi.fn(() => null),
			setItem: vi.fn(),
		} as unknown as Storage
		const store = createAppStore(client(), storage, () => 'custom:work')
		expect(store.getState().createCategory('Work')).toEqual({
			ok: true,
			id: 'custom:work',
		})
		expect(
			store.getState().categories[store.getState().categories.length - 1],
		).toMatchObject({ id: 'custom:work', label: 'Work', builtIn: false })
		store.getState().moveApp('code', 'custom:work')
		expect(
			store.getState().renameCategory('custom:work', 'Projects'),
		).toEqual({ ok: true })
		expect(store.getState().deleteCategory('custom:work')).toEqual({
			ok: true,
		})
		expect(store.getState().categoryOverrides.code).toBe('other')
		expect(store.getState().categoryOrder).not.toContain('custom:work')
	})

	it('launches and uninstalls through source-aware client calls', async () => {
		const api = client()
		const store = createAppStore(api)
		await store.getState().launch(apps[2])
		await expect(
			store.getState().uninstall('codex'),
		).resolves.toBeUndefined()
		expect(api.launchApp).toHaveBeenCalledWith({
			launchKind: 'app_user_model_id',
			path: 'OpenAI.Codex!App',
		})
		expect(api.uninstallApp).toHaveBeenCalledWith('codex')
	})
})
