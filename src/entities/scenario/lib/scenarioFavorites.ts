import type { Scenario } from '../model/scenario.types'

export function filterFavoriteScenarios(
	scenarios: readonly Scenario[],
	favoriteIds: readonly string[],
): Scenario[] {
	const favorites = new Set(favoriteIds)
	return scenarios.filter(scenario => favorites.has(scenario.id))
}
