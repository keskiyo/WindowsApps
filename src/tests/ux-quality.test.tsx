import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { App } from '../App'
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
		description: 'Code editor by Microsoft',
		canUninstall: true,
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
		getUninstallPreview: vi.fn().mockResolvedValue({
			appName: 'Visual Studio Code',
			publisher: 'Microsoft',
			source: 'registry',
			mechanism: 'registered_command',
			command: 'C:\\Code\\uninstall.exe /quiet',
		}),
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
		setScanSettings: vi.fn().mockImplementation(async s => s),
		getUninstallHistory: vi.fn().mockResolvedValue([]),
		clearUninstallHistory: vi.fn().mockResolvedValue(undefined),
		pickFolder: vi.fn().mockResolvedValue(null),
		openTelegram: vi.fn().mockResolvedValue(undefined),
		openGithub: vi.fn().mockResolvedValue(undefined),
		...systemOverrides,
	}
	const store = createAppStore(client, localStorage)
	render(<App store={store} systemClient={systemClient} />)
	return { client, store }
}

describe('UX quality — first impressions', () => {
	beforeEach(() => {
		localStorage.clear()
		document.body.style.overflow = ''
		Object.defineProperty(window, 'matchMedia', {
			configurable: true,
			value: vi.fn(() => ({
				matches: false,
				media: '',
				onchange: null,
				addEventListener: vi.fn(),
				removeEventListener: vi.fn(),
				addListener: vi.fn(),
				removeListener: vi.fn(),
				dispatchEvent: vi.fn(),
			})),
		})
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
		vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
			cb(0)
			return 1
		})
	})

	// Issue 1: double-click prevention
	// The launch button should guard against rapid repeated clicks. Without this
	// guard, two clicks fire two separate launchApp calls — launching the app twice.
	it('does not fire launchApp twice on rapid double-click', async () => {
		let resolveFirst!: () => void
		const launchApp = vi
			.fn()
			.mockImplementationOnce(
				() => new Promise<void>(res => (resolveFirst = res)),
			)
			.mockResolvedValue(undefined)
		const { client } = renderApp({ launchApp })
		const btn = await screen.findByRole('button', { name: 'Launch Steam' })
		await userEvent.dblClick(btn)
		resolveFirst()
		// After debounce/disabled guard: only one call
		expect(client.launchApp).toHaveBeenCalledTimes(1)
	})

	// Issue 2: uninstall success toast names the app
	it('shows the app name in the uninstall success toast', async () => {
		renderApp()
		await userEvent.click(
			await screen.findByRole('button', {
				name: 'Manage Visual Studio Code',
			}),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Uninstall' }),
		)
		await userEvent.click(
			await screen.findByRole('button', { name: 'Confirm uninstall' }),
		)
		expect(
			await screen.findByText('Visual Studio Code uninstalled'),
		).toBeInTheDocument()
	})

	// Issue 3: uninstall error toast names the app
	it('shows the app name in the uninstall error toast', async () => {
		renderApp({
			uninstallApp: vi
				.fn()
				.mockRejectedValue(new Error('permission denied')),
		})
		await userEvent.click(
			await screen.findByRole('button', {
				name: 'Manage Visual Studio Code',
			}),
		)
		await userEvent.click(
			screen.getByRole('menuitem', { name: 'Uninstall' }),
		)
		await userEvent.click(
			await screen.findByRole('button', { name: 'Confirm uninstall' }),
		)
		expect(
			await screen.findByText('Could not uninstall Visual Studio Code'),
		).toBeInTheDocument()
	})

	// Issue 4: launch button shows loading state
	// Without a loading/disabled state the user has no feedback that the launch is
	// in progress. The button should be disabled (or aria-busy) until the promise settles.
	it('disables the launch button while the app is launching', async () => {
		let resolveFirst!: () => void
		const launchApp = vi
			.fn()
			.mockImplementationOnce(
				() => new Promise<void>(res => (resolveFirst = res)),
			)
		renderApp({ launchApp })
		const btn = await screen.findByRole('button', { name: 'Launch Steam' })
		await userEvent.click(btn)
		// Button must be disabled or aria-busy while the promise is pending
		expect(btn).toBeDisabled()
		resolveFirst()
	})

	// Issue 5: star button is keyboard-accessible even when visually hidden (opacity-0)
	it('favorite star button is focusable and not aria-hidden when app is not favorited', async () => {
		renderApp()
		const star = await screen.findByRole('button', {
			name: 'Add Steam to favorites',
		})
		expect(star).not.toHaveAttribute('aria-hidden', 'true')
		star.focus()
		expect(star).toHaveFocus()
	})

	// Issue 6: tooltip shows description when available, falls back to path
	it('shows description as tooltip when available', async () => {
		renderApp()
		const btn = await screen.findByRole('button', {
			name: 'Launch Visual Studio Code',
		})
		expect(btn).toHaveAttribute('title', 'Code editor by Microsoft')
	})

	it('falls back to path as tooltip when description is absent', async () => {
		renderApp()
		const btn = await screen.findByRole('button', { name: 'Launch Steam' })
		expect(btn).toHaveAttribute('title', 'C:\\Steam.exe')
	})
})

describe('UX quality — keyboard & native (round 3)', () => {
	beforeEach(() => {
		localStorage.clear()
		document.body.style.overflow = ''
		Object.defineProperty(window, 'matchMedia', {
			configurable: true,
			value: vi.fn(() => ({
				matches: false,
				media: '',
				onchange: null,
				addEventListener: vi.fn(),
				removeEventListener: vi.fn(),
				addListener: vi.fn(),
				removeListener: vi.fn(),
				dispatchEvent: vi.fn(),
			})),
		})
		Object.defineProperty(Element.prototype, 'scrollIntoView', {
			configurable: true,
			value: vi.fn(),
		})
		vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
			cb(0)
			return 1
		})
	})

	// Ctrl+K command palette: open, filter, launch with Enter, close with Escape.
	it('opens the command palette with Ctrl+K and launches the selected app with Enter', async () => {
		const { client } = renderApp()
		await screen.findByText('Steam')
		await userEvent.keyboard('{Control>}k{/Control}')
		const palette = await screen.findByRole('dialog', {
			name: 'Quick launch',
		})
		expect(palette).toBeInTheDocument()
		const input = screen.getByRole('combobox', {
			name: 'Quick launch search',
		})
		await userEvent.type(input, 'steam')
		await userEvent.keyboard('{Enter}')
		expect(client.launchApp).toHaveBeenCalledWith({ id: 'steam' })
		expect(
			screen.queryByRole('dialog', { name: 'Quick launch' }),
		).not.toBeInTheDocument()
	})

	it('opens the command palette from the physical K key on non-Latin layouts', async () => {
		renderApp()
		await screen.findByText('Steam')
		fireEvent.keyDown(document, {
			key: 'л',
			code: 'KeyK',
			ctrlKey: true,
		})
		expect(
			await screen.findByRole('dialog', { name: 'Quick launch' }),
		).toBeInTheDocument()
	})

	it('closes the command palette with Escape without launching', async () => {
		const { client } = renderApp()
		await screen.findByText('Steam')
		await userEvent.keyboard('{Control>}k{/Control}')
		await screen.findByRole('dialog', { name: 'Quick launch' })
		await userEvent.keyboard('{Escape}')
		expect(
			screen.queryByRole('dialog', { name: 'Quick launch' }),
		).not.toBeInTheDocument()
		expect(client.launchApp).not.toHaveBeenCalled()
	})

	// aria-current marks the active navigation view for screen readers.
	it('marks the active navigation view with aria-current', async () => {
		renderApp()
		await screen.findByText('Steam')
		await userEvent.click(
			screen.getByRole('button', { name: 'Open navigation' }),
		)
		const allApps = screen.getByRole('button', { name: 'All Apps' })
		expect(allApps).toHaveAttribute('aria-current', 'page')
		const favorites = screen.getByRole('button', { name: /Favorites/ })
		expect(favorites).not.toHaveAttribute('aria-current')
	})

	// Closing the actions menu returns focus to the grip trigger (keyboard users keep place).
	it('returns focus to the manage button after the menu closes', async () => {
		renderApp()
		const manage = await screen.findByRole('button', {
			name: 'Manage Steam',
		})
		await userEvent.click(manage)
		await screen.findByRole('menuitem', { name: 'App info' })
		await userEvent.keyboard('{Escape}')
		expect(manage).toHaveFocus()
	})

	// Failed launch surfaces a Retry affordance instead of a dead-end toast.
	it('offers a Retry action when launching fails', async () => {
		renderApp({
			launchApp: vi.fn().mockRejectedValue(new Error('access denied')),
		})
		await userEvent.click(
			await screen.findByRole('button', { name: 'Launch Steam' }),
		)
		expect(
			await screen.findByRole('button', { name: 'Retry' }),
		).toBeInTheDocument()
	})
})
