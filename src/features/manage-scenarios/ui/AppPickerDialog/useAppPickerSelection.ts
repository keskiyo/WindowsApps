import { useMemo, useState } from 'react'
import { type AppInfo, appIdentity } from '../../../../entities/app'

export function useAppPickerSelection() {
	const [selected, setSelected] = useState<AppInfo[]>([])
	const identities = useMemo(
		() => new Set(selected.map(appIdentity)),
		[selected],
	)

	return {
		selected,
		isSelected(app: AppInfo) {
			return identities.has(appIdentity(app))
		},
		toggle(app: AppInfo) {
			const identity = appIdentity(app)
			setSelected(current =>
				current.some(entry => appIdentity(entry) === identity)
					? current.filter(entry => appIdentity(entry) !== identity)
					: [...current, app],
			)
		},
	}
}
