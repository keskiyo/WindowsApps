import { describe, expect, it, vi } from 'vitest'
import { createAppStore, selectFilteredApps } from '../../../src/app/store/appStore'
import type { AppInfo, AppsClient } from '../../../src/entities/app'

function app(index: number): AppInfo {
	const name =
		index % 4 === 0
			? `Windows Utility ${index}`
			: index % 4 === 1
				? `Кириллица приложение ${index}`
				: index % 4 === 2
					? `Duplicate App ${Math.floor(index / 2)}`
					: `Unavailable Tool ${index}`
	return {
		id: `app-${index}`,
		name,
		path: `C:\\Program Files\\${'Long Path '.repeat(12)}${index}\\App.exe`,
		category: index % 2 === 0 ? 'utilities' : 'development',
		iconBase64: index % 3 === 0 ? null : 'data:image/png;base64,x',
		launchKind: 'executable',
		sourceKind: 'registry',
		description: index % 5 === 0 ? 'Searchable metadata' : null,
		version: null,
		publisher: index % 7 === 0 ? 'Windows Apps Publisher' : null,
		installLocation: `C:\\Program Files\\App ${index}`,
		canUninstall: false,
		targetAvailability: index % 4 === 3 ? 'missing' : null,
	}
}

function client(apps: AppInfo[]): AppsClient {
	return {
		getApps: vi.fn().mockResolvedValue({ apps, hasCache: true, generation: 3 }),
		refreshApps: vi.fn().mockResolvedValue(apps),
		cancelScan: vi.fn().mockResolvedValue(undefined),
		launchApp: vi.fn().mockResolvedValue(undefined),
		closeApps: vi
			.fn()
			.mockResolvedValue({ closed: 0, notRunning: 0, unavailable: 0, failed: 0 }),
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

describe('catalog soak semantics', () => {
	it('keeps 10000 records bounded, responsive to refresh cancellation and generation-safe', async () => {
		const apps = Array.from({ length: 10000 }, (_, index) => app(index))
		const appsClient = client(apps)
		const store = createAppStore(appsClient)
		await store.getState().load()

		const queries = ['windows', 'кириллица', 'duplicate', 'metadata', 'missing']
		for (let index = 0; index < 1000; index += 1) {
			store.getState().setQuery(queries[index % queries.length])
			expect(selectFilteredApps(store.getState()).length).toBeLessThanOrEqual(
				apps.length,
			)
		}

		for (let index = 0; index < 100; index += 1)
			await Promise.all([
				store.getState().refresh(),
				store.getState().cancelScan(),
			])

		store.getState().applyPatches([
			{ id: 'app-1', generation: 2, publisher: 'Stale' },
		])

		expect(store.getState().apps).toHaveLength(10000)
		expect(store.getState().isRefreshing).toBe(false)
		expect(store.getState().apps[1].publisher).not.toBe('Stale')
		expect(appsClient.refreshApps).toHaveBeenCalledTimes(100)
		expect(appsClient.cancelScan).toHaveBeenCalledTimes(100)
	}, 30000)
})
