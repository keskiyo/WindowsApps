import {
	appIdentity,
	closeBlockedMessage,
	isCloseBlocked,
} from '../../entities/app'
import {
	MAX_SCENARIO_ENTRIES,
	MAX_SCENARIOS,
	scenarioAppSnapshot,
	type Scenario,
	type ScenarioList,
} from '../../entities/scenario'
import type {
	AppState,
	GetAppState,
	PersistPreferences,
	SetAppState,
} from './types'

interface ScenarioActionOptions {
	set: SetAppState
	get: GetAppState
	persist: PersistPreferences
	idFactory: () => string
}

type ScenarioActions = Pick<
	AppState,
	| 'createScenario'
	| 'renameScenario'
	| 'deleteScenario'
	| 'addScenarioApp'
	| 'removeScenarioApp'
	| 'toggleFavoriteScenario'
>

function nameTaken(
	scenarios: Scenario[],
	value: string,
	exceptId?: string,
): boolean {
	return scenarios.some(
		scenario =>
			scenario.id !== exceptId &&
			scenario.name.toLocaleLowerCase() === value.toLocaleLowerCase(),
	)
}

function listKey(list: ScenarioList): 'launchIdentities' | 'closeIdentities' {
	return list === 'launch' ? 'launchIdentities' : 'closeIdentities'
}

function oppositeListKey(
	list: ScenarioList,
): 'launchIdentities' | 'closeIdentities' {
	return list === 'launch' ? 'closeIdentities' : 'launchIdentities'
}

function snapshotKey(
	list: ScenarioList,
): 'launchAppSnapshots' | 'closeAppSnapshots' {
	return list === 'launch' ? 'launchAppSnapshots' : 'closeAppSnapshots'
}

export function createScenarioActions({
	set,
	get,
	persist,
	idFactory,
}: ScenarioActionOptions): ScenarioActions {
	function updateScenario(
		id: string,
		change: (scenario: Scenario) => Scenario,
	) {
		set(state => ({
			scenarios: state.scenarios.map(scenario =>
				scenario.id === id ? change(scenario) : scenario,
			),
		}))
		persist()
	}

	return {
		createScenario(name) {
			const value = name.trim()
			if (!value) return { ok: false, error: 'Enter a scenario name' }
			if (nameTaken(get().scenarios, value))
				return { ok: false, error: 'Scenario name already exists' }
			if (get().scenarios.length >= MAX_SCENARIOS)
				return { ok: false, error: 'Too many scenarios' }
			const id = idFactory()
			set(state => ({
				scenarios: [
					...state.scenarios,
					{
						id,
						name: value,
						launchIdentities: [],
						closeIdentities: [],
						launchAppSnapshots: {},
						closeAppSnapshots: {},
						createdAt: Date.now(),
					},
				],
			}))
			persist()
			return { ok: true, id }
		},
		renameScenario(id, name) {
			const value = name.trim()
			if (!value) return { ok: false, error: 'Enter a scenario name' }
			if (nameTaken(get().scenarios, value, id))
				return { ok: false, error: 'Scenario name already exists' }
			if (!get().scenarios.some(scenario => scenario.id === id))
				return { ok: false, error: 'Scenario not found' }
			updateScenario(id, scenario => ({ ...scenario, name: value }))
			return { ok: true }
		},
		deleteScenario(id) {
			set(state => ({
				scenarios: state.scenarios.filter(
					scenario => scenario.id !== id,
				),
				favoriteScenarioIds: state.favoriteScenarioIds.filter(
					entry => entry !== id,
				),
			}))
			persist()
		},
		toggleFavoriteScenario(id) {
			if (!get().scenarios.some(scenario => scenario.id === id)) return
			set(state => ({
				favoriteScenarioIds: state.favoriteScenarioIds.includes(id)
					? state.favoriteScenarioIds.filter(entry => entry !== id)
					: [...state.favoriteScenarioIds, id],
			}))
			persist()
		},
		addScenarioApp(id, list, identity) {
			const key = listKey(list)
			const oppositeKey = oppositeListKey(list)
			const snapshotsKey = snapshotKey(list)
			const app = get().apps.find(entry => appIdentity(entry) === identity)
			const scenario = get().scenarios.find(entry => entry.id === id)
			if (!scenario) return { ok: false, error: 'Scenario not found' }
			if (scenario[key].includes(identity))
				return { ok: false, error: 'Already in this list' }
			if (scenario[oppositeKey].includes(identity))
				return { ok: false, error: 'An app cannot both launch and close' }
			if (scenario[key].length >= MAX_SCENARIO_ENTRIES)
				return {
					ok: false,
					error: `A list holds at most ${MAX_SCENARIO_ENTRIES} apps`,
				}
			if (list === 'close') {
				if (app && isCloseBlocked(app))
					return { ok: false, error: closeBlockedMessage(app) }
			}
			updateScenario(id, entry => ({
				...entry,
				[key]: [...entry[key], identity],
				...(app
					? {
						[snapshotsKey]: {
							...(entry[snapshotsKey] ?? {}),
							[identity]: scenarioAppSnapshot(app),
						},
					}
					: {}),
			}))
			return { ok: true }
		},
		removeScenarioApp(id, list, identity) {
			const key = listKey(list)
			const snapshotsKey = snapshotKey(list)
			updateScenario(id, scenario => ({
				...scenario,
				[key]: scenario[key].filter(entry => entry !== identity),
				[snapshotsKey]: Object.fromEntries(
					Object.entries(scenario[snapshotsKey] ?? {}).filter(
						([entry]) => entry !== identity,
					),
				),
			}))
		},
	}
}
