import type { RefObject } from 'react'
import type { AppInfo } from '../../../../entities/app'
import type { ScenarioList } from '../../../../entities/scenario'
import type { CloseRiskMark } from '../../types'

export interface AppPickerHeaderProps {
	list: ScenarioList
	scenarioName: string
	label: string
	query: string
	inputRef: RefObject<HTMLInputElement>
	onQueryChange(value: string): void
}

export interface AppPickerRowProps {
	app: AppInfo
	caption: string
	checked: boolean
	disabled: boolean
	mark: CloseRiskMark | null
	onToggle(): void
}
