import { type AppInfo, appIdentity } from '../../app'
import type { ScenarioAppSnapshot } from '../model/scenario.types'

export interface UnavailableScenarioApp {
	identity: string
	name: string
	iconBase64: string | null
}

export interface ResolvedScenarioList {
	apps: AppInfo[]
	unavailable: UnavailableScenarioApp[]
	missing: number
}

export function resolveScenarioApps(
	identities: string[],
	apps: AppInfo[],
	snapshots: Record<string, ScenarioAppSnapshot> = {},
): ResolvedScenarioList {
	const byIdentity = new Map(apps.map(app => [appIdentity(app), app]))
	const resolved: AppInfo[] = []
	const unavailable: UnavailableScenarioApp[] = []
	for (const identity of identities) {
		const app = byIdentity.get(identity)
		if (app) resolved.push(app)
		else {
			const snapshot = snapshots[identity]
			unavailable.push({
				identity,
				name: snapshot?.name ?? 'Unavailable application',
				iconBase64: snapshot?.iconBase64 ?? null,
			})
		}
	}
	return { apps: resolved, unavailable, missing: unavailable.length }
}
