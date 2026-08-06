import { act, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { AppGrid } from '../../../src/widgets/catalog-content/ui/AppGrid/AppGrid'
import { createAppStore } from '../../../src/app/store/appStore'
import { AppStoreProvider } from '../../../src/app/store/storeContext'
import type { AppInfo, AppsClient } from '../../../src/entities/app'
import type { CategoryDefinition } from '../../../src/entities/category'
import { triggerIntersection } from '../setup'

function emptyClient(): AppsClient {
	return {
		getApps: vi.fn().mockResolvedValue({ apps: [], hasCache: true }),
		refreshApps: vi.fn().mockResolvedValue([]),
		cancelScan: vi.fn().mockResolvedValue(undefined),
		launchApp: vi.fn().mockResolvedValue(undefined),
		closeApps: vi
			.fn()
			.mockResolvedValue({ closed: 0, notRunning: 0, unavailable: 0 }),
		getAppDetails: vi.fn().mockResolvedValue({
			fileSizeBytes: null,
			fileCreatedAt: null,
			fileModifiedAt: null,
			architecture: 'unknown',
			signature: 'unavailable',
			executableExists: null,
			installLocationExists: null,
		}),
		openAppFolder: vi.fn().mockResolvedValue(undefined),
		getUninstallPreview: vi.fn().mockResolvedValue({
			appName: '',
			publisher: null,
			source: 'registry',
			mechanism: 'registered_command',
		}),
		uninstallApp: vi.fn().mockResolvedValue(undefined),
		onAppsUpdated: vi.fn().mockResolvedValue(() => undefined),
		onScanProgress: vi.fn().mockResolvedValue(() => undefined),
	}
}

function app(index: number): AppInfo {
	return {
		id: `app-${index}`,
		name: `Sample App ${index}`,
		path: `C:\\Program Files\\Sample ${index}\\Sample.exe`,
		category: 'utilities',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: `C:\\Program Files\\Sample ${index}`,
		canUninstall: false,
	}
}

const categories: CategoryDefinition[] = [
	{ id: 'utilities', label: 'Utilities', builtIn: true },
]

function renderGrid(size: number) {
	render(
		<AppStoreProvider store={createAppStore(emptyClient(), localStorage)}>
		<AppGrid
			apps={Array.from({ length: size }, (_, index) => app(index))}
			isLoading={false}
			hasQuery={false}
			activeView='all'
			categoryOrder={['utilities']}
			categories={categories}
			collapsedCategories={[]}
			favoriteAppIds={[]}
			onBack={vi.fn()}
			onToggleCategory={vi.fn()}
			onToggleFavorite={vi.fn()}
			onMoveApp={vi.fn()}
			onLaunch={vi.fn().mockResolvedValue(undefined)}
			onInfo={vi.fn()}
			onUninstall={vi.fn()}
			onHide={vi.fn()}
			onRestore={vi.fn()}
			onPromoteAuxiliary={vi.fn()}
			onDemoteAuxiliary={vi.fn()}
			onRenameCategory={vi.fn().mockReturnValue({ ok: true })}
			onDeleteCategory={vi.fn().mockReturnValue({ ok: true })}
		/>
		</AppStoreProvider>,
	)
}

function mountedCards() {
	return screen.getAllByRole('button', { name: /^Launch Sample App/ }).length
}

// `content-visibility` skips layout and paint for off-screen cards but not the mount itself: one
// DOM subtree and one useDraggable registration per card. An auto-scan of fixed drives can find
// thousands of executables, and creating all of them synchronously is what delays first
// interaction. The bound must not depend on how large the catalog is.
describe('catalog grid mounting', () => {
	it('mounts a bounded number of cards regardless of catalog size', () => {
		renderGrid(2000)
		const small = mountedCards()

		cleanupRender()
		renderGrid(8000)

		expect(small).toBeLessThan(200)
		expect(mountedCards()).toBe(small)
	})

	it('keeps the category heading reporting the full count, not the mounted batch', () => {
		renderGrid(2000)

		expect(
			screen.getByRole('group', { name: 'Utilities category controls' }),
		).toHaveTextContent('2000 apps')
		expect(mountedCards()).toBeLessThan(2000)
	})

	it('mounts the next batch when the sentinel scrolls into view', () => {
		renderGrid(2000)
		const initial = mountedCards()

		act(() => {
			triggerIntersection()
		})

		expect(mountedCards()).toBeGreaterThan(initial)
	})

	it('mounts a small catalog completely and shows no sentinel', () => {
		renderGrid(12)

		expect(mountedCards()).toBe(12)
		act(() => {
			triggerIntersection()
		})
		expect(mountedCards()).toBe(12)
	})
})

function cleanupRender() {
	document.body.innerHTML = ''
}
