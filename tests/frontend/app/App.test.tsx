import { act, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { App } from '../../../src/app/App'
import { PREFERENCES_KEY } from '../../../src/app/store/preferences'
import { createAppStore } from '../../../src/app/store/appStore'
import type { AppInfo, AppsClient } from '../../../src/entities/app'
import type { SystemClient } from '../../../src/entities/system'

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
		id: 'steam',
		name: 'Steam',
		path: 'C:\\Steam.exe',
		category: 'games',
	}),
	app({
		id: 'code',
		name: 'Visual Studio Code',
		path: 'C:\\Code.exe',
		category: 'development',
		publisher: 'Microsoft',
		description: 'Code editor',
		version: '1.99',
		canUninstall: true,
	}),
	app({
		id: 'chrome',
		name: 'Google Chrome',
		path: 'C:\\Chrome.exe',
		category: 'browsers',
	}),
]

function renderApp(
	overrides: Partial<AppsClient> = {},
	systemOverrides: Partial<SystemClient> = {},
) {
	const client: AppsClient = {
		getApps: vi.fn().mockResolvedValue({ apps, hasCache: true }),
		refreshApps: vi.fn().mockResolvedValue(apps),
		cancelScan: vi.fn().mockResolvedValue(undefined),
		launchApp: vi.fn().mockResolvedValue(undefined),
		closeApps: vi
			.fn()
			.mockResolvedValue({ closed: 0, notRunning: 0, unavailable: 0, failed: 0 }),
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
	const systemClient: SystemClient = {
		getSettings: vi.fn().mockResolvedValue({
			version: '0.1.0',
			autostartEnabled: false,
			shortcut: { available: true, label: 'Win+Shift+Q', error: null },
			scanSettings: {
				autoScanFixedDrives: true,
				includedPaths: [],
				excludedPaths: [],
			},
			fixedDrives: ['C:\\'],
		}),
		setAutostart: vi.fn().mockResolvedValue(undefined),
		setScanSettings: vi.fn().mockImplementation(async settings => settings),
		getUninstallHistory: vi.fn().mockResolvedValue([]),
		clearUninstallHistory: vi.fn().mockResolvedValue(undefined),
		pickFolder: vi.fn().mockResolvedValue(null),
		openTelegram: vi.fn().mockResolvedValue(undefined),
		openGithub: vi.fn().mockResolvedValue(undefined),
		openAppsSettings: vi.fn().mockResolvedValue(undefined),
		...systemOverrides,
	}
	const store = createAppStore(client, localStorage)
	render(
		<App store={store} systemClient={systemClient} appsClient={client} />,
	)
	return { client, store, systemClient }
}

function setDesktopNavigation(matches: boolean) {
	Object.defineProperty(window, 'matchMedia', {
		configurable: true,
		value: vi.fn(() => ({
			matches,
			media: '(min-width: 1024px)',
			onchange: null,
			addEventListener: vi.fn(),
			removeEventListener: vi.fn(),
			addListener: vi.fn(),
			removeListener: vi.fn(),
			dispatchEvent: vi.fn(),
		})),
	})
}

describe('App', () => {
	beforeEach(() => {
		setDesktopNavigation(false)
		localStorage.clear()
		document.body.style.overflow = ''
		Object.defineProperty(window, 'scrollTo', {
			configurable: true,
			value: vi.fn(),
		})
		Object.defineProperty(Element.prototype, 'scrollTo', {
			configurable: true,
			value: vi.fn(),
		})
		Object.defineProperty(Element.prototype, 'scrollIntoView', {
			configurable: true,
			value: vi.fn(),
		})
		vi.stubGlobal(
			'requestAnimationFrame',
			(callback: FrameRequestCallback) => {
				callback(0)
				return 1
			},
		)
	})

	it('uses permanent navigation at 1024px and hides the burger', async () => {
		setDesktopNavigation(true)
		renderApp()
		await screen.findByText('Steam')
		expect(
			screen.getByRole('navigation', { name: 'App navigation' }),
		).toBeInTheDocument()
		expect(
			screen.queryByRole('button', { name: 'Open navigation' }),
		).not.toBeInTheDocument()
		const settings = screen.getByRole('button', { name: 'Settings' })
		expect(settings).toHaveTextContent('Settings')
	})

	it('uses the dark Graphite Surface theme and Neon Glass app cards', async () => {
		renderApp()
		const launch = await screen.findByRole('button', {
			name: 'Launch Steam',
		})
		const card = launch.closest('article')

		expect(document.querySelector('.app-shell')).toHaveClass(
			'theme-graphite-surface',
		)
		expect(card).toHaveClass('app-card-glass')
		expect(document.getElementById('catalog-scroll')).toHaveClass(
			'overflow-x-hidden',
		)
	})

	it('renders the English catalog and category counts', async () => {
		renderApp()
		expect(
			screen.getByLabelText('Loading applications'),
		).toBeInTheDocument()
		expect(
			await screen.findByRole('heading', { name: 'Games' }),
		).toBeInTheDocument()
		expect(
			screen.getByRole('heading', { name: 'Development' }),
		).toBeInTheDocument()
		expect(screen.getAllByText('1 app')).toHaveLength(3)
	})

	it('exposes full app and category names when visible labels are truncated', async () => {
		renderApp()
		const steam = await screen.findByText('Steam')
		expect(steam).toHaveAttribute('title', 'Steam')
		expect(
			screen.getByRole('heading', { name: 'Development' }),
		).toHaveAttribute('title', 'Development')
	})

	it('shows the app version under the name and omits it when absent', async () => {
		renderApp()
		const version = await screen.findByText('v1.99')
		expect(version).toHaveAttribute('title', 'Version 1.99')
		const codeCard = screen
			.getByRole('button', { name: 'Launch Visual Studio Code' })
			.closest('article')
		expect(codeCard).toContainElement(version)
		// A version-less entry renders no version line: the launch button shows only the name.
		const steamButton = screen.getByRole('button', { name: 'Launch Steam' })
		expect(steamButton).toHaveTextContent(/^Steam$/)
	})

	it('keeps view switching in the navigation instead of repeating it above the grid', async () => {
		const { store } = renderApp()
		await screen.findByRole('button', { name: 'Launch Steam' })
		// The catalog screen is the grid alone; a second copy of the view filters used to sit
		// above it and duplicate both the sidebar and the header count.
		expect(
			screen.queryByRole('region', { name: 'Catalog filters' }),
		).not.toBeInTheDocument()

		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		const navigation = screen.getByRole('dialog', {
			name: 'App navigation',
		})
		expect(
			within(navigation).getByRole('button', { name: /^All Apps/ }),
		).toHaveAttribute('aria-current', 'page')
		await userEvent.click(
			within(navigation).getByRole('button', { name: 'Favorites 0' }),
		)
		expect(store.getState().activeView).toBe('favorites')
	})

	it('updates the header count to show search matches', async () => {
		renderApp()
		const search = await screen.findByRole('textbox', {
			name: 'Search applications',
		})
		await userEvent.type(search, 'code')
		expect(screen.getByText('1 match')).toBeInTheDocument()
	})

	it('shows and dismisses the first-run scan prompt without scanning', async () => {
		const getApps = vi.fn().mockResolvedValue({ apps: [], hasCache: false })
		const { client } = renderApp({ getApps })
		expect(
			await screen.findByText('Find your applications'),
		).toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('button', { name: 'Dismiss scan prompt' }),
		)
		expect(
			screen.queryByText('Find your applications'),
		).not.toBeInTheDocument()
		expect(client.refreshApps).not.toHaveBeenCalled()
		expect(
			screen.getByRole('button', { name: 'Scan for apps' }),
		).toBeInTheDocument()
	})

	it('filters applications from the English search field', async () => {
		renderApp()
		const search = await screen.findByPlaceholderText('Search apps…')
		await userEvent.type(search, 'chrome')
		expect(screen.queryByText('Visual Studio Code')).not.toBeInTheDocument()
		expect(screen.getByText('Google Chrome')).toBeInTheDocument()
	})

	it('launches an application from its card', async () => {
		const { client } = renderApp()
		await userEvent.click(
			await screen.findByRole('button', {
				name: 'Launch Visual Studio Code',
			}),
		)
		expect(client.launchApp).toHaveBeenCalledWith({ id: 'code' })
	})

	it('adds an app to favorites without launching it', async () => {
		const { client, store } = renderApp()
		const star = await screen.findByRole('button', {
			name: 'Add Steam to favorites',
		})
		await userEvent.click(star)
		expect(store.getState().favoriteAppIds).toEqual(['steam'])
		expect(client.launchApp).not.toHaveBeenCalled()
		expect(
			screen.getByRole('button', { name: 'Remove Steam from favorites' }),
		).toHaveAttribute('aria-pressed', 'true')
	})

	it('shows favorites in one flat grid and handles an empty list', async () => {
		const { store } = renderApp()
		await screen.findByText('Steam')
		store.getState().setActiveView('favorites')
		expect(await screen.findByText('No favorites yet')).toBeInTheDocument()
		store.getState().toggleFavorite('code')
		expect(
			await screen.findByText('Visual Studio Code'),
		).toBeInTheDocument()
		expect(
			screen.queryByRole('heading', { name: 'Development' }),
		).not.toBeInTheDocument()
	})

	it('reveals search matches from a collapsed category', async () => {
		renderApp()
		await screen.findByText('Visual Studio Code')
		await userEvent.click(
			screen.getByRole('button', { name: 'Collapse Development' }),
		)
		expect(screen.queryByText('Visual Studio Code')).not.toBeInTheDocument()
		await userEvent.type(
			screen.getByPlaceholderText('Search apps…'),
			'visual',
		)
		expect(screen.getByText('Visual Studio Code')).toBeInTheDocument()
	})

	it('renders persisted category order with sidebar reorder controls', async () => {
		setDesktopNavigation(true)
		localStorage.setItem(
			PREFERENCES_KEY,
			JSON.stringify({
				version: 1,
				categoryOrder: ['browsers', 'games'],
				favoriteAppIds: [],
				collapsedCategories: [],
			}),
		)
		renderApp()
		await screen.findByText('Google Chrome')
		const headings = screen.getAllByRole('heading', { level: 2 })
		expect(
			headings.slice(0, 2).map(heading => heading.textContent),
		).toEqual(['Browsers', 'Games'])
		expect(
			within(
				screen.getByRole('navigation', { name: 'App navigation' }),
			).getByRole('button', { name: 'Games' }),
		).toHaveAttribute('aria-roledescription', 'sortable')
	})

	it('uses the title for collapse while keeping reordering in the sidebar', async () => {
		setDesktopNavigation(true)
		renderApp()
		await screen.findByText('Steam')
		const reorder = within(
			screen.getByRole('navigation', { name: 'App navigation' }),
		).getByRole('button', { name: 'Games' })
		const toggle = screen.getByRole('button', { name: 'Collapse Games' })
		expect(toggle).toHaveTextContent('Games')
		expect(toggle).toHaveTextContent('1 app')
		expect(reorder).not.toBe(toggle)
	})

	it('hides the rename pencil while a category name is being edited', async () => {
		renderApp()
		await screen.findByText('Steam')
		await userEvent.click(
			screen.getByRole('button', { name: 'Rename Games category' }),
		)

		expect(
			screen.getByRole('textbox', { name: 'Rename Games category' }),
		).toBeInTheDocument()
		expect(
			screen.queryByRole('button', { name: 'Rename Games category' }),
		).not.toBeInTheDocument()
	})

	it('clears search and restores input focus', async () => {
		renderApp()
		const search = await screen.findByRole('textbox', {
			name: 'Search applications',
		})
		await userEvent.type(search, 'chrome')
		await userEvent.click(
			screen.getByRole('button', { name: 'Clear search' }),
		)
		expect(search).toHaveValue('')
		expect(search).toHaveFocus()
		expect(
			screen.queryByRole('button', { name: 'Clear search' }),
		).not.toBeInTheDocument()
	})

	it('moves an application from its grip menu without launching it', async () => {
		const { client, store } = renderApp()
		await userEvent.click(
			await screen.findByRole('button', {
				name: 'Manage Visual Studio Code',
			}),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		await userEvent.click(
			within(
				screen.getByRole('menu', {
					name: 'Move Visual Studio Code to category',
				}),
			).getByRole('menuitem', { name: 'AI & Agents' }),
		)
		expect(store.getState().categoryOverrides.code).toBe('ai')
		expect(client.launchApp).not.toHaveBeenCalled()
		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		expect(
			screen.getByRole('button', { name: 'AI & Agents' }),
		).toHaveTextContent('1')
	})

	it('closes the category cascade with Escape and restores the menu trigger focus', async () => {
		renderApp()
		const manage = await screen.findByRole('button', {
			name: 'Manage Visual Studio Code',
		})
		await userEvent.click(manage)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		expect(
			screen.getByRole('menu', {
				name: 'Move Visual Studio Code to category',
			}),
		).toBeInTheDocument()

		await userEvent.keyboard('{Escape}')

		expect(
			screen.queryByRole('menu', { name: 'Visual Studio Code actions' }),
		).not.toBeInTheDocument()
		expect(manage).toHaveFocus()
	})

	it('allows moving an app even when direct uninstall is unavailable', async () => {
		const { store } = renderApp()
		await userEvent.click(
			await screen.findByRole('button', { name: 'Manage Steam' }),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Browsers' }),
		)
		expect(store.getState().categoryOverrides.steam).toBe('browsers')
	})

	it('files an app into Installers & Docs and moves it back out', async () => {
		const { store } = renderApp()
		await userEvent.click(
			await screen.findByRole('button', { name: 'Manage Steam' }),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Installers & Docs' }),
		)

		// It leaves the catalog and shows up under Installers — never under Docs, which stays the
		// scanner's own verdict.
		expect(
			screen.queryByRole('button', { name: 'Launch Steam' }),
		).not.toBeInTheDocument()
		store.getState().setActiveView('installers_docs')
		const installers = await screen.findByRole('region', {
			name: 'Installers 1',
		})
		expect(
			within(installers).getByRole('button', { name: 'Launch Steam' }),
		).toBeInTheDocument()
		expect(
			screen.queryByRole('region', { name: /^Docs/ }),
		).not.toBeInTheDocument()

		// A hand-filed artifact keeps "Move to category", which is the only way back out.
		await userEvent.click(
			screen.getByRole('button', { name: 'Manage Steam' }),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		await userEvent.click(screen.getByRole('menuitem', { name: 'Games' }))

		expect(store.getState().installerAppIds).toEqual([])
		expect(store.getState().categoryOverrides.steam).toBe('games')
	})

	it('hides an app and restores it from the Hidden view', async () => {
		const { store } = renderApp()
		store.getState().toggleFavorite('steam')
		await userEvent.click(
			await screen.findByRole('button', { name: 'Manage Steam' }),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Hide from catalog' }),
		)
		expect(
			screen.queryByRole('button', { name: 'Launch Steam' }),
		).not.toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		await userEvent.click(
			within(
				screen.getByRole('dialog', { name: 'App navigation' }),
			).getByRole('button', { name: 'More' }),
		)
		const hidden = await screen.findByRole('button', { name: 'Hidden 1' })
		expect(hidden).toHaveTextContent('1')
		await userEvent.click(hidden)
		expect(
			await screen.findByRole('button', { name: 'Launch Steam' }),
		).toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('button', { name: 'Manage Steam' }),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Restore to catalog' }),
		)
		expect(store.getState().hiddenAppIds).not.toContain('steam')
		expect(store.getState().favoriteAppIds).toContain('steam')
	})

	it('restores and moves an auxiliary tool back without exposing favorites', async () => {
		const helper = app({
			id: 'helper',
			canonicalIdentity: 'identity:helper',
			name: 'Runtime Helper',
			path: String.raw`C:\Tool\runtime\helper.exe`,
			category: 'utilities',
			visibilityClass: 'auxiliary',
			visibilityReasons: ['runtime_directory', 'product_component'],
		})
		const { store } = renderApp({
			getApps: vi.fn().mockResolvedValue({ apps: [...apps, helper], hasCache: true }),
		})
		await screen.findByRole('button', { name: 'Launch Steam' })
		await userEvent.click(screen.getByRole('button', { name: 'Open navigation' }))
		await userEvent.click(
			within(screen.getByRole('dialog', { name: 'App navigation' })).getByRole(
				'button',
				{ name: 'More' },
			),
		)
		await userEvent.click(
			await screen.findByRole('button', { name: 'Auxiliary tools 1' }),
		)

		expect(
			screen.queryByRole('button', { name: 'Add Runtime Helper to favorites' }),
		).not.toBeInTheDocument()
		await userEvent.click(screen.getByRole('button', { name: 'Manage Runtime Helper' }))
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Restore to catalog' }),
		)
		store.getState().setActiveView('all')
		expect(
			await screen.findByRole('button', { name: 'Launch Runtime Helper' }),
		).toBeInTheDocument()
		await userEvent.click(screen.getByRole('button', { name: 'Manage Runtime Helper' }))
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Move back to Auxiliary tools' }),
		)

		expect(
			screen.queryByRole('button', { name: 'Launch Runtime Helper' }),
		).not.toBeInTheDocument()
		store.getState().setActiveView('auxiliary')
		expect(
			await screen.findByRole('button', { name: 'Launch Runtime Helper' }),
		).toBeInTheDocument()
	})

	// Quick launch offers what the grid shows. An auxiliary entry is an updater stub, a command
	// environment or a product component, and it carries the product's own name and icon — Discord's
	// Squirrel stub sat beside Discord itself, so the palette listed two identical rows.
	it('excludes auxiliary tools and hidden apps from the command palette', async () => {
		const helper = app({
			id: 'helper',
			name: 'Runtime Helper',
			path: String.raw`C:\Tool\runtime\helper.exe`,
			category: 'utilities',
			visibilityClass: 'auxiliary',
		})
		const hidden = app({
			id: 'hidden-helper',
			name: 'Hidden Helper',
			path: String.raw`C:\Tool\runtime\hidden.exe`,
			category: 'utilities',
			visibilityClass: 'auxiliary',
		})
		const { store } = renderApp({
			getApps: vi
				.fn()
				.mockResolvedValue({
					apps: [...apps, helper, hidden],
					hasCache: true,
				}),
		})
		await screen.findByRole('button', { name: 'Launch Steam' })
		store.getState().hideApp(hidden.id)
		await userEvent.keyboard('{Control>}k{/Control}')
		const input = screen.getByRole('combobox', {
			name: 'Quick launch search',
		})
		await userEvent.type(input, 'Runtime Helper')
		expect(screen.getByText(/No apps match/)).toBeInTheDocument()

		await userEvent.clear(input)
		await userEvent.type(input, 'Hidden Helper')
		expect(screen.getByText(/No apps match/)).toBeInTheDocument()

		// The palette still offers the catalog it is for, so the two misses above mean something.
		await userEvent.clear(input)
		await userEvent.type(input, 'Steam')
		expect(screen.getByRole('option', { name: 'Steam' })).toBeInTheDocument()
	})

	it.each(['p', 'з'])(
		'prevents the print shortcut when KeyP reports %s',
		async key => {
			renderApp()
			await screen.findByText('Steam')
			const event = new KeyboardEvent('keydown', {
				key,
				code: 'KeyP',
				ctrlKey: true,
				bubbles: true,
				cancelable: true,
			})

			document.dispatchEvent(event)

			expect(event.defaultPrevented).toBe(true)
			expect(
				screen.queryByRole('dialog', { name: 'Quick launch' }),
			).not.toBeInTheDocument()
		},
	)

	it('keeps the sticky header above cards and open app menus', async () => {
		renderApp()
		expect(screen.getByRole('banner')).toHaveClass('z-300')
		const manage = await screen.findByRole('button', {
			name: 'Manage Steam',
		})
		await userEvent.click(manage)
		const card = manage.closest('article')
		expect(card).toHaveAttribute('data-menu-open', 'true')
		expect(card).toHaveClass('z-100')
		expect(card?.closest('section')).not.toHaveClass('focus-within:z-[300]')
	})

	it('shows application information from the grip menu', async () => {
		renderApp()
		const manage = await screen.findByRole('button', {
			name: 'Manage Visual Studio Code',
		})
		await userEvent.click(
			manage,
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'App info' }),
		)
		const dialog = screen.getByRole('dialog', {
			name: 'Visual Studio Code information',
		})
		expect(dialog).toBeInTheDocument()
		// Version also renders on the card, so scope the dialog assertions to the dialog.
		expect(within(dialog).getByText('Microsoft')).toBeInTheDocument()
		expect(within(dialog).getByText('1.99')).toBeInTheDocument()
		expect(document.body.style.overflow).toBe('hidden')
		// The shell scrolls its own panel, not the document, so a body-only lock left the catalog
		// scrolling behind every dialog.
		expect(
			document.getElementById('catalog-scroll')?.style.overflowY,
		).toBe('hidden')
		await userEvent.click(
			screen.getByRole('button', { name: 'Close app information' }),
		)
		expect(document.body.style.overflow).toBe('')
		expect(document.getElementById('catalog-scroll')?.style.overflowY).toBe(
			'',
		)
		expect(manage).toHaveFocus()
	})

	it('closes the grip menu when the grip is pressed again', async () => {
		renderApp()
		const manage = await screen.findByRole('button', {
			name: 'Manage Visual Studio Code',
		})
		await userEvent.click(manage)
		expect(
			screen.getByRole('menu', { name: 'Visual Studio Code actions' }),
		).toBeInTheDocument()
		await userEvent.click(manage)
		expect(
			screen.queryByRole('menu', {
				name: 'Visual Studio Code actions',
			}),
		).not.toBeInTheDocument()
	})

	it('requires confirmation before starting uninstall', async () => {
		const { client } = renderApp()
		await userEvent.click(
			await screen.findByRole('button', {
				name: 'Manage Visual Studio Code',
			}),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Uninstall' }),
		)
		expect(client.uninstallApp).not.toHaveBeenCalled()
		expect(document.body.style.overflow).toBe('hidden')
		expect(await screen.findByText('Microsoft')).toBeInTheDocument()
		expect(screen.getByText('Registry')).toBeInTheDocument()
		expect(
			screen.getByText('Registered uninstall command'),
		).toBeInTheDocument()
		expect(
			screen.queryByText('C:\\Code\\uninstall.exe /quiet'),
		).not.toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('button', { name: 'Confirm uninstall' }),
		)
		expect(client.uninstallApp).toHaveBeenCalledWith('code')
		expect(client.refreshApps).toHaveBeenCalledTimes(1)
		expect(document.body.style.overflow).toBe('')
	})

	it('disables uninstall when no registered uninstall target exists', async () => {
		const { client } = renderApp()
		await userEvent.click(
			await screen.findByRole('button', { name: 'Manage Steam' }),
		)
		const unavailable = screen.getByRole('menuitem', {
			name: 'Uninstall unavailable',
		})
		expect(unavailable).toBeDisabled()
		expect(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		).toBeInTheDocument()
		expect(client.uninstallApp).not.toHaveBeenCalled()
		expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument()
	})

	it('keeps uninstall confirmation disabled when preview fails', async () => {
		const { client } = renderApp({
			getUninstallPreview: vi
				.fn()
				.mockRejectedValue(new Error('preview unavailable')),
		})
		await userEvent.click(
			await screen.findByRole('button', {
				name: 'Manage Visual Studio Code',
			}),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Uninstall' }),
		)

			expect(
				await screen.findByText(
					'The operation could not be completed. Try again.',
				),
			).toBeInTheDocument()
		expect(
			screen.getByRole('button', { name: 'Confirm uninstall' }),
		).toBeDisabled()
		expect(client.uninstallApp).not.toHaveBeenCalled()
	})

	it('returns to All Apps from the header without clearing search', async () => {
		const { store } = renderApp()
		await screen.findByText('Steam')
		store.getState().setQuery('steam')
		store.getState().setActiveView('favorites')
		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Go to All Apps' }),
		)
		expect(store.getState().activeView).toBe('all')
		expect(store.getState().query).toBe('steam')
		expect(
			screen.queryByRole('dialog', { name: 'App navigation' }),
		).not.toBeInTheDocument()
		expect(Element.prototype.scrollTo).toHaveBeenCalledWith({
			top: 0,
			behavior: 'smooth',
		})
	})

	it('opens the navigation drawer and closes it with Escape', async () => {
		renderApp()
		await screen.findByText('Steam')
		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		expect(
			screen.getByRole('dialog', { name: 'App navigation' }),
		).toBeInTheDocument()
		await userEvent.keyboard('{Escape}')
		expect(
			screen.queryByRole('dialog', { name: 'App navigation' }),
		).not.toBeInTheDocument()
	})

	it('opens the flat Favorites view from the drawer', async () => {
		renderApp()
		await screen.findByText('Steam')
		await userEvent.click(
			screen.getByRole('button', { name: 'Add Steam to favorites' }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		await userEvent.click(
			within(screen.getByRole('dialog', { name: 'App navigation' })).getByRole(
				'button',
				{ name: /Favorites/ },
			),
		)
		expect(await screen.findByText('Steam')).toBeInTheDocument()
		expect(
			screen.queryByRole('heading', { name: 'Games' }),
		).not.toBeInTheDocument()
	})

	// Regression: the drawer computed this count over the whole categorized catalog while the
	// sidebar computed it over the visible one, so the same nav item reported a different number
	// depending on window width. `filterVisibleApps` is authoritative — the Favorites view drops
	// hidden and auxiliary apps — so both surfaces must report what that view actually renders.
	it.each([
		['drawer', false],
		['sidebar', true],
	])(
		'counts Favorites in the %s exactly as the Favorites view filters',
		async (_surface, desktop) => {
			setDesktopNavigation(desktop)
			const { store } = renderApp()
			await screen.findByText('Steam')
			store.getState().toggleFavorite('steam')
			store.getState().hideApp('steam')
			if (!desktop)
				await userEvent.click(
					screen.getByRole('button', { name: 'Open navigation' }),
				)

			const navigation = screen.getByRole('navigation', {
				name: 'App navigation',
			})
			expect(
				within(navigation).getByRole('button', { name: 'Favorites 0' }),
			).toBeInTheDocument()

			store.getState().setActiveView('favorites')
			expect(await screen.findByText('No favorites yet')).toBeInTheDocument()
		},
	)

	it('creates an empty custom category in the drawer and deletes it from the catalog', async () => {
		renderApp()
		await screen.findByText('Steam')
		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Add category' }),
		)
		await userEvent.type(
			screen.getByRole('textbox', { name: 'New category name' }),
			'Work',
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Save category name' }),
		)
		expect(screen.getByRole('button', { name: 'Work' })).toHaveTextContent(
			'0',
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Close navigation' }),
		)
		expect(
			screen.getByRole('heading', { name: 'Work' }),
		).toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('button', { name: 'Delete Work category' }),
		)
		expect(
			screen.getByRole('alertdialog', { name: 'Delete Work category' }),
		).toBeInTheDocument()
		await userEvent.click(
			screen.getByRole('button', { name: 'Delete category' }),
		)
		expect(
			screen.queryByRole('heading', { name: 'Work' }),
		).not.toBeInTheDocument()
	})

	it('expands and navigates to a category from the drawer', async () => {
		renderApp()
		await screen.findByText('Visual Studio Code')
		await userEvent.click(
			screen.getByRole('button', { name: 'Collapse Development' }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Development' }),
		)
		expect(
			screen.queryByRole('dialog', { name: 'App navigation' }),
		).not.toBeInTheDocument()
		expect(
			await screen.findByText('Visual Studio Code'),
		).toBeInTheDocument()
		expect(Element.prototype.scrollTo).toHaveBeenCalledWith({
			top: 0,
			behavior: 'smooth',
		})
	})

	describe('running a scenario', () => {
		async function withScenario(
			launchIds: string[],
			overrides: Partial<AppsClient> = {},
		) {
			setDesktopNavigation(true)
			const rendered = renderApp(overrides)
			await screen.findByText('Steam')
			act(() => {
				const created = rendered.store.getState().createScenario('Gaming')
				if (!created.ok) throw new Error(created.error)
				for (const id of launchIds)
					rendered.store.getState().addScenarioApp(created.id, 'launch', id)
			})
			await userEvent.click(screen.getByRole('button', { name: /^More/ }))
			return rendered
		}
		it('keeps scenario outcomes out of transient notices', async () => {
			const success = vi.spyOn(toast, 'success').mockClear()
			const { client } = await withScenario(['steam', 'code'])

			await userEvent.click(screen.getByRole('button', { name: 'Run Gaming' }))

			await waitFor(() => expect(client.launchApp).toHaveBeenCalledTimes(2))
			expect(success).not.toHaveBeenCalled()
		})
		it('keeps failed scenario outcomes out of transient notices', async () => {
			const errors = vi.spyOn(toast, 'error').mockClear()
			const success = vi.spyOn(toast, 'success').mockClear()
			await withScenario(['steam', 'code'], {
				launchApp: vi.fn().mockRejectedValue(new Error('access denied')),
			})

			await userEvent.click(screen.getByRole('button', { name: 'Run Gaming' }))

			await waitFor(() => expect(errors).not.toHaveBeenCalled())
			expect(success).not.toHaveBeenCalled()
		})
	})
})
