import { useState } from 'react'
import {
	type AppInfo,
	appIdentity,
	closeRiskBadge,
	closeRiskReason,
	closeRiskWarning,
} from '../../../entities/app'
import type { Scenario, ScenarioList } from '../../../entities/scenario'
import type { CloseRiskMark, ScenarioNameResult } from '../types'

interface PickerOptions {
	scenario: Scenario
	onAddApp(
		id: string,
		list: ScenarioList,
		identity: string,
	): ScenarioNameResult
}

const OTHER_LIST_NOTE: Record<ScenarioList, string> = {
	launch: 'Already in the close list',
	close: 'Already in the launch list',
}

export function useScenarioAppPicker({ scenario, onAddApp }: PickerOptions) {
	const [picking, setPicking] = useState<ScenarioList | null>(null)
	const [pending, setPending] = useState<AppInfo[]>([])
	const [error, setError] = useState<string | null>(null)

	function identitiesOf(list: ScenarioList) {
		return list === 'launch'
			? scenario.launchIdentities
			: scenario.closeIdentities
	}

	function commit(list: ScenarioList, apps: AppInfo[]) {
		let failure: string | null = null
		for (const app of apps) {
			const result = onAddApp(scenario.id, list, appIdentity(app))
			if (!result.ok && !failure) failure = result.error
		}
		setError(failure)
	}

	function markOf(app: AppInfo): CloseRiskMark | null {
		const badge = closeRiskBadge(app)
		const reason = closeRiskReason(app)
		return badge && reason ? { badge, reason } : null
	}

	return {
		picking,
		error,
		markOf,
		open: setPicking,
		dismissPicker: () => setPicking(null),
		candidates(apps: AppInfo[]) {
			if (!picking) return []
			const added = new Set(identitiesOf(picking))
			return apps.filter(app => !added.has(appIdentity(app)))
		},
		noteOf(app: AppInfo): string | null {
			if (!picking) return null
			const other = picking === 'launch' ? 'close' : 'launch'
			return identitiesOf(other).includes(appIdentity(app))
				? OTHER_LIST_NOTE[picking]
				: null
		},
		confirming: pending[0] ?? null,
		confirmationMessage: pending[0] ? closeRiskWarning(pending[0]) : null,
		cancelConfirmation: () => setPending(queue => queue.slice(1)),
		confirm(selected: AppInfo[]) {
			const list = picking
			setPicking(null)
			if (!list) return
			if (list === 'launch') return commit('launch', selected)
			commit(
				'close',
				selected.filter(app => !closeRiskWarning(app)),
			)
			setPending(selected.filter(app => closeRiskWarning(app)))
		},
		acceptConfirmation() {
			if (pending[0]) commit('close', [pending[0]])
			setPending(queue => queue.slice(1))
		},
	}
}
