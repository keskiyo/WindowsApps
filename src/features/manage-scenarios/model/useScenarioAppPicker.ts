import { useState } from 'react'
import {
	type AppInfo,
	appIdentity,
	closeRiskBadge,
	closeRiskReason,
	closeRiskWarning,
} from '../../../entities/app'
import type { ScenarioList } from '../../../entities/scenario'
import type { CloseRiskMark, ScenarioNameResult } from '../types'

interface PickerOptions {
	scenarioId: string
	onAddApp(
		id: string,
		list: ScenarioList,
		identity: string,
	): ScenarioNameResult
}

export function useScenarioAppPicker({
	scenarioId,
	onAddApp,
}: PickerOptions) {
	const [picking, setPicking] = useState<ScenarioList | null>(null)
	const [confirming, setConfirming] = useState<AppInfo | null>(null)
	const [error, setError] = useState<string | null>(null)

	function commit(list: ScenarioList, app: AppInfo) {
		const result = onAddApp(scenarioId, list, appIdentity(app))
		setError(result.ok ? null : result.error)
	}

	function markOf(app: AppInfo): CloseRiskMark | null {
		const badge = closeRiskBadge(app)
		const reason = closeRiskReason(app)
		return badge && reason ? { badge, reason } : null
	}

	return {
		picking,
		confirming,
		error,
		markOf,
		open: setPicking,
		dismissPicker: () => setPicking(null),
		cancelConfirmation: () => setConfirming(null),
		confirmationMessage: confirming ? closeRiskWarning(confirming) : null,
		select(app: AppInfo) {
			const list = picking
			setPicking(null)
			if (!list) return
			if (list === 'close' && closeRiskWarning(app)) {
				setError(null)
				setConfirming(app)
				return
			}
			commit(list, app)
		},
		acceptConfirmation() {
			if (confirming) commit('close', confirming)
			setConfirming(null)
		},
	}
}
