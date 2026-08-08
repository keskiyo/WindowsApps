import {
	appIdentity,
	closeBlockedMessage,
	isCloseBlocked,
} from '../../entities/app'
import {
	MAX_SCENARIO_ENTRIES,
	MAX_SCENARIOS,
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
			const scenario = get().scenarios.find(entry => entry.id === id)
			if (!scenario) return { ok: false, error: 'Scenario not found' }
			if (scenario[key].includes(identity))
				return { ok: false, error: 'Already in this list' }
			if (scenario[key].length >= MAX_SCENARIO_ENTRIES)
				return {
					ok: false,
					error: `A list holds at most ${MAX_SCENARIO_ENTRIES} apps`,
				}
			if (list === 'close') {
				const app = get().apps.find(
					entry => appIdentity(entry) === identity,
				)
				if (app && isCloseBlocked(app))
					return { ok: false, error: closeBlockedMessage(app) }
			}
			updateScenario(id, entry => ({
				...entry,
				[key]: [...entry[key], identity],
			}))
			return { ok: true }
		},
		removeScenarioApp(id, list, identity) {
			const key = listKey(list)
			updateScenario(id, scenario => ({
				...scenario,
				[key]: scenario[key].filter(entry => entry !== identity),
			}))
		},
	}
}
