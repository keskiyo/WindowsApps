import type { CatalogChangeSummary } from '../types'

export function catalogChangeMessage(
	summary: CatalogChangeSummary,
): string | null {
	const total = summary.added + summary.removed + summary.updated
	if (!total) return null
	if (summary.added > 0 && !summary.removed && !summary.updated)
		return `${summary.added} application${summary.added === 1 ? '' : 's'} added`
	if (summary.removed > 0 && !summary.added && !summary.updated)
		return `${summary.removed} application${summary.removed === 1 ? '' : 's'} removed`
	return 'Application catalog updated'
}
