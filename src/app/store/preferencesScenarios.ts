import {
	MAX_SCENARIO_ENTRIES,
	MAX_SCENARIOS,
	type Scenario,
	type ScenarioAppSnapshot,
	normalizeScenarioAppSnapshot,
} from '../../entities/scenario'
import { uniqueStrings } from './preferencesFields'

function normalizeScenarioSnapshots(
	value: unknown,
	identities: string[],
): Record<string, ScenarioAppSnapshot> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return {}
	const snapshots = value as Record<string, unknown>
	return Object.fromEntries(
		identities.flatMap(identity => {
			const snapshot = normalizeScenarioAppSnapshot(snapshots[identity])
			return snapshot ? [[identity, snapshot]] : []
		}),
	)
}

export function normalizeScenarios(value: unknown): Scenario[] {
	if (!Array.isArray(value)) return []
	const seenIds = new Set<string>()
	const scenarios: Scenario[] = []
	for (const item of value.slice(0, MAX_SCENARIOS)) {
		if (!item || typeof item !== 'object' || Array.isArray(item)) continue
		const raw = item as Record<string, unknown>
		const id = typeof raw.id === 'string' ? raw.id.trim() : ''
		const name = typeof raw.name === 'string' ? raw.name.trim() : ''
		if (!id || !name || seenIds.has(id)) continue
		seenIds.add(id)
		const createdAt = raw.createdAt
		const launchIdentities = uniqueStrings(raw.launchIdentities).slice(
			0,
			MAX_SCENARIO_ENTRIES,
		)
		const launchAppSnapshots = normalizeScenarioSnapshots(
			raw.launchAppSnapshots,
			launchIdentities,
		)
		const closeIdentities = uniqueStrings(raw.closeIdentities)
			.filter(identity => !launchIdentities.includes(identity))
			.slice(0, MAX_SCENARIO_ENTRIES)
		const closeAppSnapshots = normalizeScenarioSnapshots(
			raw.closeAppSnapshots,
			closeIdentities,
		)
		scenarios.push({
			id,
			name,
			launchIdentities,
			closeIdentities,
			launchAppSnapshots,
			closeAppSnapshots,
			createdAt:
				typeof createdAt === 'number' &&
				Number.isFinite(createdAt) &&
				createdAt > 0
					? createdAt
					: null,
		})
	}
	return scenarios
}
