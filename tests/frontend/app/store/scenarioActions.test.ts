import { describe, expect, it, vi } from 'vitest'
import { PREFERENCES_KEY } from '../../../../src/app/store/preferences'
import { createAppStore } from '../../../../src/app/store/appStore'
import {
	MAX_SCENARIO_ENTRIES,
	MAX_SCENARIOS,
} from '../../../../src/entities/scenario'
import type { AppsClient } from '../../../../src/entities/app'

function client(): AppsClient {
	return {
		getApps: vi.fn().mockResolvedValue({ apps: [], hasCache: true }),
		refreshApps: vi.fn().mockResolvedValue([]),
		cancelScan: vi.fn().mockResolvedValue(undefined),
		launchApp: vi.fn().mockResolvedValue(undefined),
		closeApps: vi
			.fn()
			.mockResolvedValue({ closed: 0, notRunning: 0, unavailable: 0 }),
		getAppDetails: vi.fn(),
		openAppFolder: vi.fn().mockResolvedValue(undefined),
		getUninstallPreview: vi.fn(),
		uninstallApp: vi.fn().mockResolvedValue(undefined),
		onAppsUpdated: vi.fn().mockResolvedValue(() => undefined),
		onScanProgress: vi.fn().mockResolvedValue(() => undefined),
	}
}

function memoryStorage() {
	const values = new Map<string, string>()
	return {
		values,
		storage: {
			getItem: (key: string) => values.get(key) ?? null,
			setItem: (key: string, value: string) => void values.set(key, value),
		} as unknown as Storage,
	}
}

let counter = 0
const idFactory = () => `scenario-${(counter += 1)}`

describe('scenario actions', () => {
	it('creates, renames and deletes a scenario', () => {
		const store = createAppStore(client(), memoryStorage().storage, idFactory)

		const created = store.getState().createScenario('  Gaming  ')
		expect(created).toEqual({ ok: true, id: expect.any(String) })
		expect(store.getState().scenarios[0]?.name).toBe('Gaming')
		// The creation date is what the More card shows; only a real one is any use.
		expect(store.getState().scenarios[0]?.createdAt).toBeGreaterThan(0)

		const id = created.ok ? created.id : ''
		expect(store.getState().renameScenario(id, 'Focus')).toEqual({ ok: true })
		expect(store.getState().scenarios[0]?.name).toBe('Focus')

		store.getState().deleteScenario(id)
		expect(store.getState().scenarios).toEqual([])
	})

	it('refuses a blank or duplicate name', () => {
		const store = createAppStore(client(), memoryStorage().storage, idFactory)
		store.getState().createScenario('Gaming')

		expect(store.getState().createScenario('   ')).toEqual({
			ok: false,
			error: 'Enter a scenario name',
		})
		// Case-insensitive: two scenarios called Gaming and gaming are one name to a reader.
		expect(store.getState().createScenario('gaming')).toEqual({
			ok: false,
			error: 'Scenario name already exists',
		})
		expect(store.getState().scenarios).toHaveLength(1)
	})

	it('adds an app to each list and removes it again', () => {
		const store = createAppStore(client(), memoryStorage().storage, idFactory)
		const created = store.getState().createScenario('Gaming')
		const id = created.ok ? created.id : ''

		expect(store.getState().addScenarioApp(id, 'launch', 'app:game')).toEqual({
			ok: true,
		})
		expect(store.getState().addScenarioApp(id, 'close', 'app:chat')).toEqual({
			ok: true,
		})
		expect(store.getState().scenarios[0]).toMatchObject({
			launchIdentities: ['app:game'],
			closeIdentities: ['app:chat'],
		})

		store.getState().removeScenarioApp(id, 'launch', 'app:game')
		expect(store.getState().scenarios[0]?.launchIdentities).toEqual([])
		// Removing from one list must not touch the other.
		expect(store.getState().scenarios[0]?.closeIdentities).toEqual(['app:chat'])
	})

	it('reports a duplicate rather than adding the same app twice', () => {
		const store = createAppStore(client(), memoryStorage().storage, idFactory)
		const created = store.getState().createScenario('Gaming')
		const id = created.ok ? created.id : ''
		store.getState().addScenarioApp(id, 'launch', 'app:game')

		expect(store.getState().addScenarioApp(id, 'launch', 'app:game')).toEqual({
			ok: false,
			error: 'Already in this list',
		})
		expect(store.getState().scenarios[0]?.launchIdentities).toEqual([
			'app:game',
		])
	})

	// One click runs the whole list, so what a list can hold is bounded at the source too.
	it('caps a list and the number of scenarios', () => {
		const store = createAppStore(client(), memoryStorage().storage, idFactory)
		const created = store.getState().createScenario('Gaming')
		const id = created.ok ? created.id : ''
		for (let entry = 0; entry < MAX_SCENARIO_ENTRIES; entry += 1)
			store.getState().addScenarioApp(id, 'launch', `app:${entry}`)

		expect(store.getState().addScenarioApp(id, 'launch', 'app:extra')).toEqual({
			ok: false,
			error: `A list holds at most ${MAX_SCENARIO_ENTRIES} apps`,
		})

		for (let index = 1; index < MAX_SCENARIOS; index += 1)
			store.getState().createScenario(`Scenario ${index}`)
		expect(store.getState().createScenario('One too many')).toEqual({
			ok: false,
			error: 'Too many scenarios',
		})
		expect(store.getState().scenarios).toHaveLength(MAX_SCENARIOS)
	})

	it('persists a scenario and reloads it', () => {
		const { storage, values } = memoryStorage()
		const store = createAppStore(client(), storage, idFactory)
		const created = store.getState().createScenario('Gaming')
		const id = created.ok ? created.id : ''
		store.getState().addScenarioApp(id, 'close', 'app:chat')

		expect(
			JSON.parse(values.get(PREFERENCES_KEY) ?? '{}').scenarios,
		).toEqual([
			{
				id,
				name: 'Gaming',
				launchIdentities: [],
				closeIdentities: ['app:chat'],
				createdAt: expect.any(Number),
			},
		])
		expect(createAppStore(client(), storage).getState().scenarios).toEqual(
			store.getState().scenarios,
		)
	})
})
