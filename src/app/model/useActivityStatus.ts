import type { AppInfo } from '../../entities/app'

interface ActivityInput {
	apps: AppInfo[]
	launchingIds: string[]
	isRefreshing: boolean
}

export function useActivityStatus({
	apps,
	launchingIds,
	isRefreshing,
}: ActivityInput) {
	const launchingName =
		launchingIds.length === 1
			? apps.find(app => app.id === launchingIds[0])?.name
			: undefined
	const label =
		launchingIds.length > 1
			? `Launching ${launchingIds.length} apps…`
			: launchingName
				? `Launching ${launchingName}…`
				: isRefreshing
					? 'Scanning applications…'
					: ''
	return { label, active: launchingIds.length > 0 || isRefreshing }
}
