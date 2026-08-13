import type { AppInfo } from '../../app'
import type { ScenarioAppSnapshot } from '../model/scenario.types'

export const MAX_SCENARIO_SNAPSHOT_ICON_BYTES = 32 * 1024

function snapshotIcon(value: unknown): string | null {
	return typeof value === 'string' &&
		value.length <= MAX_SCENARIO_SNAPSHOT_ICON_BYTES &&
		/^data:image\/[a-z0-9.+-]+;base64,[A-Za-z0-9+/=]*$/i.test(value)
		? value
		: null
}

export function normalizeScenarioAppSnapshot(
	value: unknown,
): ScenarioAppSnapshot | null {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return null
	const raw = value as Record<string, unknown>
	const name = typeof raw.name === 'string' ? raw.name.trim() : ''
	return name ? { name, iconBase64: snapshotIcon(raw.iconBase64) } : null
}

export function scenarioAppSnapshot(
	app: Pick<AppInfo, 'name' | 'iconBase64'>,
): ScenarioAppSnapshot {
	return {
		name: app.name.trim() || 'Unavailable application',
		iconBase64: snapshotIcon(app.iconBase64),
	}
}
