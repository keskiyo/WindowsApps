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
		resetCatalogCache: vi.fn().mockResolvedValue([apps[2]]),
		hydrateVisibleIcons: vi.fn().mockResolvedValue(undefined),
		cancelScan: vi.fn().mockResolvedValue(undefined),
		launchApp: vi.fn().mockResolvedValue(undefined),
		getUninstallPreview: vi.fn().mockResolvedValue({
			appName: 'Visual Studio Code',
			publisher: 'Microsoft',
			source: 'registry',
			mechanism: 'registered_command',
			command: 'uninstall.exe',
		}),
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

	it('marks an app launching and clears it on the ceiling timer', async () => {
		vi.useFakeTimers()
		try {
			const store = createAppStore(client())
			await store.getState().launch(apps[0])
			expect(store.getState().launchingIds).toContain('code')
			vi.advanceTimersByTime(12000)
			expect(store.getState().launchingIds).not.toContain('code')
		} finally {
			vi.useRealTimers()
		}
	})

	it('clears launching immediately when a launch fails', async () => {
		const store = createAppStore(
			client({ launchApp: vi.fn().mockRejectedValue(new Error('nope')) }),
		)
		await expect(store.getState().launch(apps[0])).rejects.toThrow()
		expect(store.getState().launchingIds).not.toContain('code')
	})

	it('clearLaunching is idempotent', () => {
		const store = createAppStore(client())
		store.getState().markLaunching('x')
		expect(store.getState().launchingIds).toEqual(['x'])
		store.getState().clearLaunching('x')
		store.getState().clearLaunching('x')
		expect(store.getState().launchingIds).toEqual([])
	})

	it('reuses an in-flight initialization so dev StrictMode does not start two scans', async () => {
		const api = client({
			startBackgroundSync: vi.fn().mockResolvedValue(undefined),
			onCatalogDelta: vi.fn().mockResolvedValue(() => undefined),
			onCatalogPatches: vi.fn().mockResolvedValue(() => undefined),
			onCatalogChanged: vi.fn().mockResolvedValue(() => undefined),
		})
		const store = createAppStore(api)

		const [firstDispose, secondDispose] = await Promise.all([
			store.getState().initialize(),
			store.getState().initialize(),
		])

		expect(api.getApps).toHaveBeenCalledOnce()
		expect(api.startBackgroundSync).toHaveBeenCalledOnce()
		expect(api.onAppsUpdated).toHaveBeenCalledOnce()
		firstDispose()
		expect(api.onAppsUpdated).toHaveBeenCalledOnce()
		secondDispose()
	})

	it('keeps one app per id when cached and updated data repeats entries', async () => {
		const duplicate = { ...apps[0] }
		const store = createAppStore(
			client({
				getApps: vi.fn().mockResolvedValue({
					apps: [apps[0], duplicate, apps[1]],
					hasCache: true,
				}),
			}),
		)

		await store.getState().load()
		store.getState().replaceApps([apps[0], duplicate, apps[1]])
		store.getState().applyDelta({
			generation: 1,
			upserted: [apps[0], duplicate],
			removedIds: [],
			summary: { added: 0, removed: 0, updated: 0 },
		})

		expect(store.getState().apps.map(app => app.id)).toEqual([
			'code',
			'chrome',
		])
	})

	it('collapses stale shortcut and executable duplicates by canonical id', () => {
		const canonicalId = 'target:d:\\games\\battle.net\\battle.net.exe'
		const shortcut = app({
			id: canonicalId,
			name: 'Battle.net',
			path: String.raw`C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Battle.net\Battle.net.lnk`,
			category: 'games',
			launchKind: 'shortcut',
			sourceKind: 'start_menu',
		})
		const executable = app({
			id: canonicalId,
			name: 'Battle.net',
			path: String.raw`D:\Games\Battle.net\Battle.net.exe`,
			category: 'games',
			launchKind: 'executable',
			sourceKind: 'portable',
		})
		const store = createAppStore(client())
		store.setState({ apps: [executable, shortcut] })

		expect(
			selectVisibleApps(store.getState()).map(item => item.id),
		).toEqual([canonicalId])
	})

	it('does not merge product-family siblings in the UI', () => {
		const shortcut = app({
			id: 's',
			name: 'Acme',
			path: String.raw`C:\Menu\Acme.lnk`,
			category: 'other',
			launchKind: 'shortcut',
		})
		const executable = app({
			id: 'e',
			name: 'Acme Launcher',
			path: String.raw`C:\Apps\Acme\Acme.exe`,
			category: 'other',
		})
		const sibling = app({
			id: 'x',
			name: 'Acme Launcher',
			path: String.raw`D:\Copy\Acme.exe`,
			category: 'other',
		})
		const store = createAppStore(client())
		store.setState({ apps: [shortcut, executable, sibling] })

		expect(
			selectVisibleApps(store.getState()).map(item => item.id),
		).toEqual(['s', 'e', 'x'])
	})

	it('keeps unresolved launcher executable when only names imply a duplicate', () => {
		const shortcut = app({
			id: 'wow-lnk',
			name: 'World of Warcraft',
			path: String.raw`C:\ProgramData\Microsoft\Windows\Start Menu\Programs\World of Warcraft\World of Warcraft.lnk`,
			category: 'games',
			launchKind: 'shortcut',
			sourceKind: 'start_menu',
		})
		const executable = app({
			id: 'wow-launcher',
			name: 'World of Warcraft Launcher',
			path: String.raw`D:\Games\World of Warcraft\World of Warcraft Launcher.exe`,
			category: 'games',
			launchKind: 'executable',
			sourceKind: 'portable',
		})
		const store = createAppStore(client())
		store.setState({ apps: [executable, shortcut] })

		expect(
			selectVisibleApps(store.getState()).map(item => item.id),
		).toEqual(['wow-launcher', 'wow-lnk'])
	})

	it('keeps unresolved Steam and executable entries when ids differ', () => {
		const steam = app({
			id: 'hearthstone-steam',
			name: 'Hearthstone',
			path: 'steam://rungameid/123',
			category: 'games',
			sourceKind: 'steam',
		})
		const executable = app({
			id: 'hearthstone-exe',
			name: 'Hearthstone',
			path: String.raw`D:\Games\Hearthstone\Hearthstone.exe`,
			category: 'games',
			sourceKind: 'portable',
		})
		const store = createAppStore(client())
		store.setState({ apps: [executable, steam] })

		expect(
			selectVisibleApps(store.getState()).map(item => item.id),
		).toEqual(['hearthstone-exe', 'hearthstone-steam'])
	})

	it('keeps unresolved TablePlus shortcut and versioned executable duplicates', () => {
		const shortcut = app({
			id: 'tableplus-lnk',
			name: 'TablePlus',
			path: String.raw`C:\ProgramData\Microsoft\Windows\Start Menu\Programs\TablePlus\TablePlus.lnk`,
			category: 'other',
			launchKind: 'shortcut',
			sourceKind: 'start_menu',
			publisher: 'TablePlus Inc',
			version: '6.4.0.0',
		})
		const executable = app({
			id: 'tableplus-exe',
			name: 'TablePlus 6.4.0',
			path: String.raw`D:\Tools\TablePlus\TablePlus.exe`,
			category: 'other',
			launchKind: 'executable',
			sourceKind: 'registry',
			publisher: 'TablePlus, Inc',
			version: '6.4.0',
		})
		const store = createAppStore(client())
		store.setState({ apps: [executable, shortcut] })

		expect(
			selectVisibleApps(store.getState()).map(item => item.id),
		).toEqual(['tableplus-exe', 'tableplus-lnk'])
	})

	it('keeps unresolved shortcut and executable duplicates when ids differ', () => {
		const shortcut = app({
			id: 'assistant-lnk',
			name: 'Assistant',
			path: String.raw`C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Assistant\Assistant.lnk`,
			category: 'other',
			launchKind: 'shortcut',
			sourceKind: 'start_menu',
			publisher: 'Vendor LLC',
			version: '5.6.2408.0',
		})
		const executable = app({
			id: 'assistant-exe',
			name: 'Assistant 5.6.2.1',
			path: String.raw`D:\Tools\Assistant\AstUtil.exe`,
			category: 'other',
			launchKind: 'executable',
			sourceKind: 'registry',
			publisher: 'Vendor',
			version: '5.6.2403.1202',
		})
		const store = createAppStore(client())
		store.setState({ apps: [executable, shortcut] })

		expect(
			selectVisibleApps(store.getState()).map(item => item.id),
		).toEqual(['assistant-exe', 'assistant-lnk'])
	})

	it('keeps same-name apps when publishers conflict', () => {
		const first = app({
			id: 'app-a',
			name: 'Assistant',
			path: String.raw`D:\A\Assistant.exe`,
			category: 'ai',
			publisher: 'Vendor A',
		})
		const second = app({
			id: 'app-b',
			name: 'Assistant',
			path: String.raw`D:\B\Assistant.exe`,
			category: 'ai',
			publisher: 'Vendor B',
		})
		const store = createAppStore(client())
		store.setState({ apps: [first, second] })

		expect(
			selectVisibleApps(store.getState()).map(item => item.id),
		).toEqual(['app-a', 'app-b'])
	})

	it('filters applications case-insensitively', () => {
		const store = createAppStore(client())
		store.setState({ apps, query: 'CHROME' })
		expect(selectFilteredApps(store.getState())).toEqual([apps[1]])
	})

	it('resets catalog cache through the client and replaces apps', async () => {
		const api = client()
		const store = createAppStore(api)

		await store.getState().resetCatalogCache()

		expect(api.resetCatalogCache).toHaveBeenCalledOnce()
		expect(store.getState().apps).toEqual([apps[2]])
		expect(store.getState().hasCache).toBe(true)
	})

	it('requests priority hydration for visible icon ids', async () => {
		const api = client()
		const store = createAppStore(api)

		await store.getState().hydrateVisibleIcons(['code', 'chrome'])

		expect(api.hydrateVisibleIcons).toHaveBeenCalledWith(['code', 'chrome'])
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
		expect(api.launchApp).toHaveBeenCalledWith({ id: 'codex' })
		expect(api.uninstallApp).toHaveBeenCalledWith('codex')
	})
})
