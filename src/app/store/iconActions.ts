import type { AppsClient } from '../../entities/app'
import type { AppState, GetAppState } from './types'

const HYDRATION_BATCH_SIZE = 128

interface IconActionOptions {
	get: GetAppState
	client: AppsClient
}

type IconActions = Pick<
	AppState,
	'hydrateVisibleIcons' | 'clearIconCache' | 'repairMissingIcons'
>

export function createIconActions({
	get,
	client,
}: IconActionOptions): IconActions {
	async function hydrateIds(ids: string[]): Promise<void> {
		if (!client.hydrateVisibleIcons) return
		for (let start = 0; start < ids.length; start += HYDRATION_BATCH_SIZE) {
			try {
				await client.hydrateVisibleIcons(
					ids.slice(start, start + HYDRATION_BATCH_SIZE),
				)
			} catch (ignored) {
				void ignored
			}
		}
	}

	return {
		async hydrateVisibleIcons(ids) {
			if (!ids.length || !client.hydrateVisibleIcons) return
			await hydrateIds(ids)
		},
		async clearIconCache() {
			if (!client.clearIconCache) return
			await client.clearIconCache()
			await get().hydrateVisibleIcons(get().apps.map(app => app.id))
		},
		async repairMissingIcons() {
			await get().hydrateVisibleIcons(
				get()
					.apps.filter(app => !app.iconBase64)
					.map(app => app.id),
			)
		},
	}
}
