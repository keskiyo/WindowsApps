import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { App } from '../App'
import { PREFERENCES_KEY } from '../lib/preferences'
import { createAppStore } from '../store/appStore'
import type { AppInfo, AppsClient, SystemClient } from '../types'

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
		uninstallApp: vi.fn().mockResolvedValue(undefined),
		onAppsUpdated: vi.fn().mockResolvedValue(() => undefined),
		onScanProgress: vi.fn().mockResolvedValue(() => undefined),
		...overrides,
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
		pickFolder: vi.fn().mockResolvedValue(null),
		openTelegram: vi.fn().mockResolvedValue(undefined),
		...systemOverrides,
	}
	const store = createAppStore(client, localStorage)
	render(<App store={store} systemClient={systemClient} />)
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
		expect(client.launchApp).toHaveBeenCalledWith({
			launchKind: 'executable',
			path: 'C:\\Code.exe',
		})
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

	it('renders persisted category order with accessible drag handles', async () => {
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
			screen.getByRole('button', { name: 'Move Games category' }),
		).toBeInTheDocument()
	})

	it('uses the category heading as the drag activator and keeps collapse separate', async () => {
		renderApp()
		await screen.findByText('Steam')
		const move = screen.getByRole('button', { name: 'Move Games category' })
		expect(move).toHaveTextContent('Games')
		expect(move).toHaveTextContent('1 app')
		expect(screen.getByRole('button', { name: 'Collapse Games' })).not.toBe(
			move,
		)
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
			screen.getByRole('menuitem', { name: 'AI & Agents' }),
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
		const hidden = screen.getByRole('button', { name: /Hidden/ })
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
		await userEvent.click(
			await screen.findByRole('button', {
				name: 'Manage Visual Studio Code',
			}),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'App info' }),
		)
		expect(
			screen.getByRole('dialog', {
				name: 'Visual Studio Code information',
			}),
		).toBeInTheDocument()
		expect(screen.getByText('Microsoft')).toBeInTheDocument()
		expect(screen.getByText('1.99')).toBeInTheDocument()
		expect(document.body.style.overflow).toBe('hidden')
		await userEvent.click(
			screen.getByRole('button', { name: 'Close app information' }),
		)
		expect(document.body.style.overflow).toBe('')
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
		expect(window.scrollTo).toHaveBeenCalledWith({
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
		await userEvent.click(screen.getByRole('button', { name: /Favorites/ }))
		expect(await screen.findByText('Steam')).toBeInTheDocument()
		expect(
			screen.queryByRole('heading', { name: 'Games' }),
		).not.toBeInTheDocument()
	})

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
		expect(Element.prototype.scrollIntoView).toHaveBeenCalledWith({
			behavior: 'smooth',
			block: 'center',
		})
	})
})
