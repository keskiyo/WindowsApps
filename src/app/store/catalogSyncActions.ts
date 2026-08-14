import { mergeIcon, reconcileFirstSeen, reconcileMarks } from './reconciliation'
import type { AppsClient } from '../../entities/app'
import type {
	AppState,
	GetAppState,
	PersistPreferences,
	SetAppState,
} from './types'

interface CatalogSyncOptions {
	set: SetAppState
	get: GetAppState
	client: AppsClient
	persist: PersistPreferences
}

type CatalogSyncActions = Pick<
	AppState,
	| 'replaceApps'
	| 'applyDelta'
	| 'applyPatches'
	| 'clearCatalogChange'
	| 'subscribe'
	| 'subscribeScanProgress'
>

export function createCatalogSyncActions({
	set,
	get,
	client,
	persist,
}: CatalogSyncOptions): CatalogSyncActions {
	return {
		replaceApps(apps) {
			const previousFirstSeen = get().firstSeenAt
			const firstSeenAt = reconcileFirstSeen(
				apps,
				previousFirstSeen,
				Date.now(),
			)
			set(state => {
				const previous = new Map(state.apps.map(app => [app.id, app]))
				return {
					apps: apps.map(app => mergeIcon(previous.get(app.id), app)),
					firstSeenAt,
				}
			})
			const marks = reconcileMarks(get(), get().apps)
			if (marks) set(marks)
			if (firstSeenAt !== previousFirstSeen || marks) persist()
		},
		applyDelta(delta) {
			if (delta.generation < get().catalogGeneration) return
			const previousFirstSeen = get().firstSeenAt
			set(state => {
				const removed = new Set(delta.removedIds)
				const apps = new Map(
					state.apps
						.filter(app => !removed.has(app.id))
						.map(app => [app.id, app]),
				)
				for (const app of delta.upserted)
					apps.set(app.id, mergeIcon(apps.get(app.id), app))
				const merged = [...apps.values()]
				return {
					apps: merged,
					catalogGeneration: delta.generation,
					firstSeenAt: reconcileFirstSeen(
						merged,
						state.firstSeenAt,
						Date.now(),
					),
				}
			})
			const marks = reconcileMarks(get(), get().apps)
			if (marks) set(marks)
			if (get().firstSeenAt !== previousFirstSeen || marks) persist()
		},
		applyPatches(patches) {
			const generation = get().catalogGeneration
			const current = patches.filter(
				patch => patch.generation === generation,
			)
			if (!current.length) return
			const byId = new Map(current.map(patch => [patch.id, patch]))
			set(state => ({
				apps: state.apps.map(app => {
					const patch = byId.get(app.id)
					if (!patch) return app
					const { id: _id, generation: _generation, ...fields } = patch
					return { ...app, ...fields }
				}),
			}))
		},
		clearCatalogChange() {
			set({ catalogChange: null })
		},
		subscribe() {
			return client.onAppsUpdated(apps => set({ apps }))
		},
		subscribeScanProgress() {
			return client.onScanProgress(scanProgress => set({ scanProgress }))
		},
	}
}
