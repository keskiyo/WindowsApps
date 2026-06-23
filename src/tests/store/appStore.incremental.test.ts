import { describe, expect, it, vi } from 'vitest'
import { createAppStore } from '../../store/appStore'
import type { AppInfo, AppsClient } from '../../types'

const code: AppInfo = {
	id: 'code',
	name: 'Visual Studio Code',
	path: 'C:\\Code.exe',
	iconBase64: null,
	category: 'development',
	launchKind: 'executable',
	sourceKind: 'registry',
	description: null,
	version: null,
	publisher: null,
	installLocation: null,
	canUninstall: false,
}

function client(): AppsClient {
	return {
		getApps: vi.fn().mockResolvedValue({
			apps: [code],
			hasCache: true,
			generation: 2,
		}),
		refreshApps: vi.fn().mockResolvedValue([code]),
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
	}
}

describe('incremental app store updates', () => {
	it('merges hydration patches without replacing catalog state', async () => {
		const store = createAppStore(client())
		await store.getState().load()

		store.getState().applyPatches([
			{
				id: 'code',
				generation: 2,
				iconBase64: 'data:image/png;base64,x',
				publisher: 'Microsoft',
			},
		])

		expect(store.getState().apps[0]).toMatchObject({
			id: 'code',
			iconBase64: 'data:image/png;base64,x',
			publisher: 'Microsoft',
		})
	})

	it('ignores stale patches and patches for removed applications', async () => {
		const store = createAppStore(client())
		await store.getState().load()

		store.getState().applyPatches([
			{ id: 'code', generation: 1, publisher: 'Stale' },
			{ id: 'missing', generation: 2, publisher: 'Missing' },
		])

		expect(store.getState().apps).toEqual([code])
	})

	it('applies stable-id deltas while keeping preferences', async () => {
		const store = createAppStore(client())
		await store.getState().load()
		store.getState().toggleFavorite('code')

		store.getState().applyDelta({
			generation: 3,
			upserted: [{ ...code, version: '2.0' }],
			removedIds: [],
			summary: { added: 0, removed: 0, updated: 1 },
		})

		expect(store.getState().apps[0].version).toBe('2.0')
		expect(store.getState().favoriteAppIds).toEqual(['code'])
	})
})
