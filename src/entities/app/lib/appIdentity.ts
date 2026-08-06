import type { AppInfo } from '../model/app.types'

/**
 * The key every durable per-app choice is stored under. A catalog id is a function of the
 * deduplication grouping and changes between releases; `preferenceIdentity` (falling back to the
 * canonical identity, then the id) is what actually survives a Force full scan or a dedup rule
 * change, so favorites, hidden apps, manual categories and first-seen stamps all key on this.
 */
export function appIdentity(app: AppInfo): string {
	return app.preferenceIdentity ?? app.canonicalIdentity ?? app.id
}
