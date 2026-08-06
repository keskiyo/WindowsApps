import { type AppInfo, appIdentity } from '../../app'

export interface ResolvedScenarioList {
	apps: AppInfo[]
	missing: number
}

export function resolveScenarioApps(
	identities: string[],
	apps: AppInfo[],
): ResolvedScenarioList {
	const byIdentity = new Map(apps.map(app => [appIdentity(app), app]))
	const resolved: AppInfo[] = []
	let missing = 0
	for (const identity of identities) {
		const app = byIdentity.get(identity)
		if (app) resolved.push(app)
		else missing += 1
	}
	return { apps: resolved, missing }
}
