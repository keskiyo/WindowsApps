import { describe, expect, it, vi } from 'vitest'
import { PREFERENCES_KEY } from '../../../src/lib/preferences'
import { AppClientError } from '../../../src/lib/clientError'
import {
	createAppStore,
	selectFilteredApps,
	selectVisibleApps,
} from '../../../src/store/appStore'
import type { AppInfo, AppsClient } from '../../../src/types'

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
		}),
		uninstallApp: vi.fn().mockResolvedValue(undefined),
		onAppsUpdated: vi.fn().mockResolvedValue(() => undefined),
		onScanProgress: vi.fn().mockResolvedValue(() => undefined),
		...overrides,
		getAppDetails:
			overrides.getAppDetails ??
			vi.fn().mockResolvedValue({
				fileSizeBytes: null,
				fileCreatedAt: null,
				fileModifiedAt: null,
				architecture: 'unknown',
				signature: 'unavailable',
				executableExists: null,
				installLocationExists: null,
			}),
		openAppFolder:
			overrides.openAppFolder ?? vi.fn().mockResolvedValue(undefined),
	}
}

describe('app store', () => {
	it('keeps auxiliary tools out of the normal catalog and search', () => {
		const store = createAppStore(client())
		store.setState({
			apps: [
				...apps,
				app({
					id: 'iconv',
					name: 'iconv',
					path: String.raw`C:\Git\usr\bin\iconv.exe`,
					category: 'development',
					visibilityClass: 'auxiliary',
					visibilityScore: -30,
					visibilityReasons: ['product_component'],
				}),
			],
			query: 'iconv',
		})

		expect(selectVisibleApps(store.getState()).map(item => item.id)).not.toContain(
			'iconv',
		)
		expect(selectFilteredApps(store.getState())).toEqual([])
	})

	it('shows hidden auxiliary tools only in the Hidden view', () => {
		const tool = app({
			id: 'iconv',
			name: 'iconv',
			path: String.raw`C:\Git\usr\bin\iconv.exe`,
			category: 'development',
			visibilityClass: 'auxiliary',
		})
		const store = createAppStore(client())
		store.setState({
			apps: [tool],
			hiddenAppIds: [tool.id],
			activeView: 'auxiliary',
		})

		expect(selectVisibleApps(store.getState())).toEqual([])
		store.setState({ activeView: 'hidden' })
		expect(selectVisibleApps(store.getState()).map(item => item.id)).toEqual([
			'iconv',
		])
	})

	// Favorites are keyed by canonicalIdentity, so a favorite survives a release that changes an
	// app's id (its id is a function of the dedup grouping, the identity is stable).
	it('keeps a favorite when the app id changes but its identity does not', async () => {
		const storage = localStorage
		storage.clear()
		const before = app({
			id: 'target:old',
			name: 'Editor',
			path: String.raw`C:\Editor.exe`,
			category: 'development',
			canonicalIdentity: 'identity:editor',
		})
		const store = createAppStore(
			client({ getApps: vi.fn().mockResolvedValue({ apps: [before], hasCache: true }) }),
			storage,
		)
		store.setState({ apps: [before] })
		store.getState().toggleFavorite('target:old')

		expect(
			JSON.parse(storage.getItem(PREFERENCES_KEY) ?? '{}').favoriteAppIdentities,
		).toEqual(['identity:editor'])

		// A later release loads the same app under a different id.
		const after = { ...before, id: 'target:new' }
		const reopened = createAppStore(
			client({ getApps: vi.fn().mockResolvedValue({ apps: [after], hasCache: true }) }),
			storage,
		)
		await reopened.getState().load()

		expect(reopened.getState().favoriteAppIds).toEqual(['target:new'])
		expect(
			selectVisibleApps({ ...reopened.getState(), activeView: 'favorites' }).map(
				item => item.id,
			),
		).toEqual(['target:new'])
	})

	it('keeps an app hidden across an id change', async () => {
		const storage = localStorage
		storage.clear()
		const before = app({
			id: 'target:old',
			name: 'Helper',
			path: String.raw`C:\Helper.exe`,
			category: 'development',
			canonicalIdentity: 'identity:helper',
		})
		const store = createAppStore(
			client({ getApps: vi.fn().mockResolvedValue({ apps: [before], hasCache: true }) }),
			storage,
		)
		store.setState({ apps: [before] })
		store.getState().hideApp('target:old')

		const after = { ...before, id: 'target:new' }
		const reopened = createAppStore(
			client({ getApps: vi.fn().mockResolvedValue({ apps: [after], hasCache: true }) }),
			storage,
		)
		await reopened.getState().load()

		expect(reopened.getState().hiddenAppIds).toEqual(['target:new'])
	})

	// A manual category override is keyed by canonicalIdentity, so it survives a Force full scan /
	// Reset cache / dedup change that renames the app id.
	it('keeps a manual category override when the app id changes but its identity does not', async () => {
		const storage = localStorage
		storage.clear()
		const before = app({
			id: 'target:old',
			name: 'Toolbox',
			path: String.raw`C:\Toolbox.exe`,
			category: 'other',
			canonicalIdentity: 'identity:toolbox',
		})
		const store = createAppStore(
			client({ getApps: vi.fn().mockResolvedValue({ apps: [before], hasCache: true }) }),
			storage,
		)
		store.setState({ apps: [before] })
		store.getState().moveApp('target:old', 'utilities')

		expect(
			JSON.parse(storage.getItem(PREFERENCES_KEY) ?? '{}')
				.categoryOverrideIdentities,
		).toEqual({ 'identity:toolbox': 'utilities' })

		// A rescan loads the same app under a different id; the override must still apply.
		const after = { ...before, id: 'target:new' }
		const reopened = createAppStore(
			client({ getApps: vi.fn().mockResolvedValue({ apps: [after], hasCache: true }) }),
			storage,
		)
		await reopened.getState().load()

		const categorized = selectVisibleApps({
			...reopened.getState(),
			activeView: 'all',
		})
		expect(
			categorized.find(item => item.id === 'target:new')?.category,
		).toBe('utilities')
	})

	it('migrates a v6 canonical collision only to the card named by its saved id', async () => {
		localStorage.setItem(
			PREFERENCES_KEY,
			JSON.stringify({
				version: 6,
				favoriteAppIds: ['cmd-shortcut'],
				favoriteAppIdentities: ['product:command-prompt'],
				hiddenAppIds: ['cmd-shortcut'],
				hiddenAppIdentities: ['product:command-prompt'],
				categoryOverrides: { 'cmd-shortcut': 'utilities' },
				categoryOverrideIdentities: {
					'product:command-prompt': 'utilities',
				},
			}),
		)
		const shortcut = app({
			id: 'cmd-shortcut',
			name: 'Command Prompt',
			path: String.raw`C:\Menu\Command Prompt.lnk`,
			category: 'system',
			canonicalIdentity: 'product:command-prompt',
			preferenceIdentity: 'preference:cmd-shortcut',
			launchKind: 'shortcut',
			sourceKind: 'start_menu',
		})
		const executable = app({
			id: 'cmd-executable',
			name: 'Command Prompt',
			path: String.raw`C:\Windows\System32\cmd.exe`,
			category: 'system',
			canonicalIdentity: 'product:command-prompt',
			preferenceIdentity: 'preference:cmd-executable',
		})
		const store = createAppStore(
			client({
				getApps: vi.fn().mockResolvedValue({
					apps: [shortcut, executable],
					hasCache: true,
				}),
			}),
			localStorage,
		)

		await store.getState().load()

		expect(store.getState().favoriteAppIds).toEqual(['cmd-shortcut'])
		expect(store.getState().hiddenAppIds).toEqual(['cmd-shortcut'])
		expect(store.getState().categoryOverrides).toEqual({
			'cmd-shortcut': 'utilities',
		})
		expect(store.getState().favoriteAppIdentities).toEqual([
			'preference:cmd-shortcut',
		])
	})

	it('keeps an ambiguous v6 canonical preference unresolved instead of fanning it out', async () => {
		localStorage.setItem(
			PREFERENCES_KEY,
			JSON.stringify({
				version: 6,
				favoriteAppIdentities: ['product:command-prompt'],
			}),
		)
		const collision = ['shortcut', 'executable'].map(role =>
			app({
				id: `cmd-${role}`,
				name: 'Command Prompt',
				path: `C:\\cmd-${role}.exe`,
				category: 'system',
				canonicalIdentity: 'product:command-prompt',
				preferenceIdentity: `preference:cmd-${role}`,
			}),
		)
		const store = createAppStore(
			client({
				getApps: vi.fn().mockResolvedValue({
					apps: collision,
					hasCache: true,
				}),
			}),
			localStorage,
		)

		await store.getState().load()

		expect(store.getState().favoriteAppIds).toEqual([])
		expect(
			JSON.parse(localStorage.getItem(PREFERENCES_KEY) ?? '{}')
				.legacyCanonicalPreferences.favorite,
		).toEqual(['product:command-prompt'])
	})

	it('persists an auxiliary tool promoted by the user', () => {
		const storage = localStorage
		storage.clear()
		const store = createAppStore(client(), storage)
		store.setState({
			apps: [
				app({
					id: 'iconv',
					name: 'iconv',
					path: String.raw`C:\Git\usr\bin\iconv.exe`,
					category: 'development',
					visibilityClass: 'auxiliary',
				}),
			],
		})

		store.getState().promoteAuxiliary('iconv')

		expect(selectVisibleApps(store.getState()).map(item => item.id)).toEqual([
			'iconv',
		])
		expect(JSON.parse(storage.getItem(PREFERENCES_KEY) ?? '{}')).toMatchObject({
			promotedAppIdentities: ['iconv'],
		})
	})

	// A refused write must reach the UI: the grid keeps showing the change either way, so
	// without this flag the user loses favorites and hidden apps with no explanation.
	it('flags preferences as unsaved when storage refuses the write', () => {
		const storage = {
			getItem: () => null,
			setItem: () => {
				throw new Error('QuotaExceededError')
			},
		} as unknown as Storage
		const store = createAppStore(client(), storage)
		store.setState({
			apps: [
				app({
					id: 'editor',
					name: 'Editor',
					path: String.raw`C:\Editor.exe`,
					category: 'development',
				}),
			],
		})

		expect(store.getState().preferencesPersisted).toBe(true)

		store.getState().toggleFavorite('editor')

		expect(store.getState().preferencesPersisted).toBe(false)
		expect(store.getState().favoriteAppIds).toEqual(['editor'])
	})

	it('migrates a legacy promoted id and survives a launcher source change', async () => {
		localStorage.setItem(
			PREFERENCES_KEY,
			JSON.stringify({ promotedAppIds: ['old-launcher'] }),
		)
		const first = app({
			id: 'old-launcher',
			canonicalIdentity: 'identity:example',
			name: 'Example Tool',
			path: String.raw`C:\Example\Tool.exe`,
			category: 'utilities',
			visibilityClass: 'auxiliary',
		})
		const replacement = app({
			...first,
			id: 'new-shortcut',
			path: String.raw`C:\Menu\Example Tool.lnk`,
			launchKind: 'shortcut',
			sourceKind: 'start_menu',
		})
		const store = createAppStore(
			client({ getApps: vi.fn().mockResolvedValue({ apps: [first], hasCache: true }) }),
			localStorage,
		)

		await store.getState().load()
		store.getState().replaceApps([replacement])

		expect(selectVisibleApps(store.getState()).map(item => item.id)).toEqual([
			'new-shortcut',
		])
		expect(JSON.parse(localStorage.getItem(PREFERENCES_KEY) ?? '{}')).toMatchObject({
			promotedAppIds: ['old-launcher'],
			promotedAppIdentities: ['identity:example'],
		})
		expect(store.getState().promotedAppIds).toEqual(['old-launcher'])
	})

	it('moves a promoted tool back to auxiliary and removes its favorite', () => {
		const tool = app({
			id: 'helper',
			canonicalIdentity: 'identity:helper',
			name: 'Helper',
			path: String.raw`C:\Tool\helper.exe`,
			category: 'utilities',
			visibilityClass: 'auxiliary',
		})
		const store = createAppStore(client())
		store.setState({ apps: [tool] })
		store.getState().promoteAuxiliary(tool.id)
		store.getState().toggleFavorite(tool.id)

		store.getState().demoteAuxiliary(tool.id)

		expect(selectVisibleApps(store.getState())).toEqual([])
		expect(store.getState().favoriteAppIds).not.toContain(tool.id)
		expect(store.getState().promotedAppIdentities).not.toContain(
			'identity:helper',
		)
	})

	it('refuses to favorite an auxiliary tool before user promotion', () => {
		const tool = app({
			id: 'helper',
			name: 'Helper',
			path: String.raw`C:\Tool\helper.exe`,
			category: 'utilities',
			visibilityClass: 'auxiliary',
		})
		const store = createAppStore(client())
		store.setState({ apps: [tool] })

		store.getState().toggleFavorite(tool.id)

		expect(store.getState().favoriteAppIds).toEqual([])
	})

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

	// Registration used to be a bare sequence of awaits: a rejection partway through left every
	// earlier listener attached with nothing owning its teardown, and the rejected promise was
	// cached, so the app could never recover from a transient bridge failure.
	it('detaches earlier listeners when a later subscription fails', async () => {
		const disposeDelta = vi.fn()
		const disposePatches = vi.fn()
		const api = client({
			onCatalogDelta: vi.fn().mockResolvedValue(disposeDelta),
			onCatalogPatches: vi.fn().mockResolvedValue(disposePatches),
			onCatalogChanged: vi
				.fn()
				.mockRejectedValue(new Error('bridge unavailable')),
		})
		const store = createAppStore(api)

		await expect(store.getState().initialize()).rejects.toThrow(
			'bridge unavailable',
		)

		expect(disposeDelta).toHaveBeenCalledOnce()
		expect(disposePatches).toHaveBeenCalledOnce()
		expect(api.onAppsUpdated).not.toHaveBeenCalled()
	})

	it('detaches every earlier listener when the last subscription fails', async () => {
		const disposers = [vi.fn(), vi.fn(), vi.fn(), vi.fn(), vi.fn()]
		const api = client({
			onCatalogDelta: vi.fn().mockResolvedValue(disposers[0]),
			onCatalogPatches: vi.fn().mockResolvedValue(disposers[1]),
			onCatalogChanged: vi.fn().mockResolvedValue(disposers[2]),
			onAppsUpdated: vi.fn().mockResolvedValue(disposers[3]),
			onScanProgress: vi.fn().mockResolvedValue(disposers[4]),
			onLaunchStatus: vi.fn().mockRejectedValue(new Error('late failure')),
		})
		const store = createAppStore(api)

		await expect(store.getState().initialize()).rejects.toThrow(
			'late failure',
		)

		for (const dispose of disposers) expect(dispose).toHaveBeenCalledOnce()
		// No catalog load ran, so the failure cannot be mistaken for an empty catalog.
		expect(api.getApps).not.toHaveBeenCalled()
	})

	it('retries initialization after a failed subscription instead of caching the rejection', async () => {
		const onCatalogChanged = vi
			.fn()
			.mockRejectedValueOnce(new Error('bridge unavailable'))
			.mockResolvedValue(() => undefined)
		const api = client({
			onCatalogDelta: vi.fn().mockResolvedValue(() => undefined),
			onCatalogPatches: vi.fn().mockResolvedValue(() => undefined),
			onCatalogChanged,
		})
		const store = createAppStore(api)

		await expect(store.getState().initialize()).rejects.toThrow(
			'bridge unavailable',
		)
		const dispose = await store.getState().initialize()

		expect(onCatalogChanged).toHaveBeenCalledTimes(2)
		expect(api.getApps).toHaveBeenCalledOnce()
		expect(store.getState().apps).toHaveLength(apps.length)
		dispose()
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

	it('batches clear and repair hydration for catalogs above the IPC limit', async () => {
		const catalog = Array.from({ length: 129 }, (_, index) =>
			app({
				id: `app-${index}`,
				name: `App ${index}`,
				path: `C:\\Apps\\app-${index}.exe`,
				category: 'other',
			}),
		)
		const hydrateVisibleIcons = vi.fn().mockResolvedValue(undefined)
		const api = client({
			clearIconCache: vi.fn().mockResolvedValue(undefined),
			hydrateVisibleIcons,
		})
		const store = createAppStore(api)
		store.setState({ apps: catalog })

		await store.getState().clearIconCache()

		expect(hydrateVisibleIcons).toHaveBeenNthCalledWith(
			1,
			catalog.slice(0, 128).map(item => item.id),
		)
		expect(hydrateVisibleIcons).toHaveBeenNthCalledWith(2, ['app-128'])

		hydrateVisibleIcons.mockClear()
		await store.getState().repairMissingIcons()

		expect(hydrateVisibleIcons).toHaveBeenNthCalledWith(
			1,
			catalog.slice(0, 128).map(item => item.id),
		)
		expect(hydrateVisibleIcons).toHaveBeenNthCalledWith(2, ['app-128'])
	})

	it('continues best-effort hydration after one batch fails', async () => {
		const catalog = Array.from({ length: 129 }, (_, index) =>
			app({
				id: `app-${index}`,
				name: `App ${index}`,
				path: `C:\\Apps\\app-${index}.exe`,
				category: 'other',
			}),
		)
		const hydrateVisibleIcons = vi
			.fn()
			.mockRejectedValueOnce(new Error('first batch failed'))
			.mockResolvedValue(undefined)
		const store = createAppStore(
			client({ hydrateVisibleIcons }),
		)
		store.setState({ apps: catalog })

		await store.getState().repairMissingIcons()

		expect(hydrateVisibleIcons).toHaveBeenCalledTimes(2)
		expect(hydrateVisibleIcons).toHaveBeenLastCalledWith(['app-128'])
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

	it('does not publish scan cancellation through the global error state', async () => {
		const cancellation = new AppClientError(
			'SCAN_CANCELLED',
			'Application scan cancelled.',
		)
		const store = createAppStore(
			client({
				refreshApps: vi.fn().mockRejectedValue(cancellation),
			}),
		)

		await expect(store.getState().refresh()).rejects.toBe(cancellation)

		expect(store.getState().error).toBeNull()
	})

	// An action reports its failure by rejecting, and its caller owns the message. `error` is the
	// background channel for work nobody awaits; writing both produced two toasts for one failure.
	it('rejects a failed launch without also writing the background error state', async () => {
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
		expect(store.getState().error).toBeNull()
		expect(store.getState().launchingIds).toEqual([])
	})

	it('rejects a failed refresh without also writing the background error state', async () => {
		const store = createAppStore(
			client({
				refreshApps: vi.fn().mockRejectedValue(new Error('scan failed')),
			}),
		)
		await expect(store.getState().refresh()).rejects.toThrow('scan failed')
		expect(store.getState().error).toBeNull()
		expect(store.getState().isRefreshing).toBe(false)
	})

	it('rejects a failed uninstall without also writing the background error state', async () => {
		const store = createAppStore(
			client({
				uninstallApp: vi.fn().mockRejectedValue(new Error('denied')),
			}),
		)
		await expect(store.getState().uninstall(apps[0].id)).rejects.toThrow(
			'denied',
		)
		expect(store.getState().error).toBeNull()
	})

	// A failed catalog load has no caller waiting on it, so it must still reach the user.
	it('still surfaces a background load failure through the error state', async () => {
		const store = createAppStore(
			client({
				getApps: vi.fn().mockRejectedValue(new Error('cache unreadable')),
			}),
		)

		await store.getState().load()

		expect(store.getState().error).toBe(
			'The operation could not be completed. Try again.',
		)
		expect(store.getState().isLoading).toBe(false)
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
		expect(store.getState().categoryOrder[0]).toBe('custom:work')
		expect(
			store.getState().categories[store.getState().categories.length - 1],
		).toMatchObject({
			id: 'custom:work',
			label: 'Work',
			builtIn: false,
			accent: expect.any(String),
		})
		expect(storage.setItem).toHaveBeenLastCalledWith(
			PREFERENCES_KEY,
			expect.stringContaining('"accent"'),
		)
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
