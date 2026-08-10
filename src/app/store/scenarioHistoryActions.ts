import { MAX_SCENARIO_HISTORY, type ScenarioRunRecord } from '../../entities/scenario'
import type { AppState, GetAppState, PersistPreferences, SetAppState } from './types'

interface ScenarioHistoryActionOptions {
	set: SetAppState
	get: GetAppState
	persist: PersistPreferences
}

export function createScenarioHistoryActions({
	set,
	get,
	persist,
}: ScenarioHistoryActionOptions): Pick<AppState, 'recordScenarioRun'> {
	return {
		recordScenarioRun(record: ScenarioRunRecord) {
			if (get().scenarioHistory.some(entry => entry.id === record.id)) return
			set(state => ({
				scenarioHistory: [record, ...state.scenarioHistory]
					.sort((left, right) => right.finishedAt - left.finishedAt)
					.slice(0, MAX_SCENARIO_HISTORY),
			}))
			persist()
		},
	}
}
