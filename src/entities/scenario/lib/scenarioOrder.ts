/**
 * Newest first, undated last.
 *
 * Scenarios are stored in the order they were created, which puts the newest one at the bottom of
 * a list the user reads from the top. The order comes from the creation date rather than from
 * reversing the array, because a scenario saved before that date existed carries none and cannot
 * honestly claim to be the newest.
 */
export function sortScenariosByNewest<T extends { createdAt: number | null }>(
	scenarios: readonly T[],
): T[] {
	return [...scenarios].sort(
		(left, right) => (right.createdAt ?? 0) - (left.createdAt ?? 0),
	)
}
