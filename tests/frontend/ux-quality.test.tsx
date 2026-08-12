import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { toast } from 'sonner'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { App } from '../../src/app/App'
import { createAppStore } from '../../src/app/store/appStore'
import type { AppInfo, AppsClient } from '../../src/entities/app'
import type { SystemClient } from '../../src/entities/system'

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
		setScanSettings: vi.fn().mockImplementation(async s => s),
		getUninstallHistory: vi.fn().mockResolvedValue([]),
		clearUninstallHistory: vi.fn().mockResolvedValue(undefined),
		savePreferencesBackup: vi.fn().mockResolvedValue(true),
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

	// Issue 6: the hover tooltip shows the clean application name (useful when the
	// visible label is truncated), not executable/version/engine metadata.
	it('shows the application name as the tooltip', async () => {
		renderApp()
		const btn = await screen.findByRole('button', {
			name: 'Launch Visual Studio Code',
		})
		expect(btn).toHaveAttribute('title', 'Visual Studio Code')
	})

	it('uses the name as tooltip regardless of executable metadata', async () => {
		renderApp()
		const btn = await screen.findByRole('button', { name: 'Launch Steam' })
		expect(btn).toHaveAttribute('title', 'Steam')
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
		const navigation = screen.getByRole('dialog', {
			name: 'App navigation',
		})
		const allApps = within(navigation).getByRole('button', {
			name: /^All Apps/,
		})
		expect(allApps).toHaveAttribute('aria-current', 'page')
		const favorites = within(navigation).getByRole('button', {
			name: /Favorites/,
		})
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

	it('does not expose launch failure internals in the toast', async () => {
		renderApp({
			launchApp: vi
				.fn()
				.mockRejectedValue(new Error('C:\\Users\\Example\\private-launch-detail')),
		})
		await userEvent.click(
			await screen.findByRole('button', { name: 'Launch Steam' }),
		)
		expect(
			await screen.findByText('Could not launch Steam'),
		).toBeInTheDocument()
		expect(
			screen.queryByText(/private-launch-detail/),
		).not.toBeInTheDocument()
	})

	// A modal dialog owes the keyboard user their place back. The palette read
	// `document.activeElement` *after* focusing its own input, so the element it saved to
	// restore was the input itself — by cleanup time already detached, leaving focus on <body>.
	it('returns focus to the trigger after the command palette closes', async () => {
		renderApp()
		const trigger = await screen.findByRole('button', {
			name: 'Launch Steam',
		})
		trigger.focus()
		expect(trigger).toHaveFocus()

		await userEvent.keyboard('{Control>}k{/Control}')
		await screen.findByRole('dialog', { name: 'Quick launch' })
		await userEvent.keyboard('{Escape}')

		expect(
			screen.queryByRole('dialog', { name: 'Quick launch' }),
		).not.toBeInTheDocument()
		expect(trigger).toHaveFocus()
	})

	it('keeps focus inside the palette while it is open', async () => {
		renderApp()
		await screen.findByText('Steam')
		await userEvent.keyboard('{Control>}k{/Control}')
		const dialog = await screen.findByRole('dialog', {
			name: 'Quick launch',
		})

		expect(
			screen.getByRole('combobox', { name: 'Quick launch search' }),
		).toHaveFocus()
		await userEvent.tab()
		expect(dialog.contains(document.activeElement)).toBe(true)
	})

	// One failed operation, one message. The store also wrote the failure into its global
	// `error`, which App turned into a second toast beside the contextual one from
	// useAppFeedback — the same problem reported twice, once without any Retry affordance.
	it('reports a failed launch exactly once', async () => {
		// spyOn returns the existing spy when the method is already wrapped, so the history has
		// to be cleared or earlier tests in this file inflate the count.
		const errorToast = vi.spyOn(toast, 'error').mockClear()
		renderApp({
			launchApp: vi.fn().mockRejectedValue(new Error('access denied')),
		})
		await userEvent.click(
			await screen.findByRole('button', { name: 'Launch Steam' }),
		)
		await screen.findByText('Could not launch Steam')

		expect(errorToast).toHaveBeenCalledTimes(1)
	})

	it('reports a failed refresh exactly once', async () => {
		// spyOn returns the existing spy when the method is already wrapped, so the history has
		// to be cleared or earlier tests in this file inflate the count.
		const errorToast = vi.spyOn(toast, 'error').mockClear()
		renderApp({
			refreshApps: vi.fn().mockRejectedValue(new Error('scan failed')),
		})
		await screen.findByText('Steam')
		await userEvent.click(
			screen.getByRole('button', { name: 'Scan for apps' }),
		)
		await screen.findByText('Could not refresh the application list')

		expect(errorToast).toHaveBeenCalledTimes(1)
	})

	it('reports a failed uninstall exactly once', async () => {
		// spyOn returns the existing spy when the method is already wrapped, so the history has
		// to be cleared or earlier tests in this file inflate the count.
		const errorToast = vi.spyOn(toast, 'error').mockClear()
		renderApp({
			uninstallApp: vi.fn().mockRejectedValue(new Error('denied')),
		})
		await userEvent.click(
			await screen.findByRole('button', {
				name: 'Manage Visual Studio Code',
			}),
		)
		await userEvent.click(
			await screen.findByRole('menuitem', { name: /Uninstall/i }),
		)
		await userEvent.click(
			await screen.findByRole('button', { name: 'Confirm uninstall' }),
		)
		await screen.findByText('Could not uninstall Visual Studio Code')

		expect(errorToast).toHaveBeenCalledTimes(1)
	})

	// The All Apps badge counted every primary app including the ones the user had hidden, so the
	// number disagreed with the cards on screen the moment anything was hidden.
	it('counts only the cards the grid actually shows', async () => {
		renderApp()
		await screen.findByText('Steam')
		async function allAppsLabel() {
			await userEvent.click(
				screen.getByRole('button', { name: 'Open navigation' }),
			)
			const navigation = screen.getByRole('dialog', {
				name: 'App navigation',
			})
			const label = within(navigation)
				.getByRole('button', { name: /^All Apps/ })
				.getAttribute('aria-label')
			await userEvent.keyboard('{Escape}')
			return label
		}

		expect(await allAppsLabel()).toBe('All Apps 2')

		await userEvent.click(
			screen.getByRole('button', { name: 'Manage Steam' }),
		)
		await userEvent.click(
			await screen.findByRole('menuitem', { name: /Hide/i }),
		)

		expect(await allAppsLabel()).toBe('All Apps 1')
	})
})
