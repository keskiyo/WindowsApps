export function sortScenariosByNewest<T extends { createdAt: number | null }>(
	scenarios: readonly T[],
): T[] {
	return [...scenarios].sort(
		(left, right) => (right.createdAt ?? 0) - (left.createdAt ?? 0),
	)
}
