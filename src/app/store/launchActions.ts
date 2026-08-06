import type { AppsClient } from '../../entities/app'
import type { AppState, GetAppState, SetAppState } from './types'

const LAUNCH_CEILING_MS = 12000

interface LaunchActionOptions {
	set: SetAppState
	get: GetAppState
	client: AppsClient
}

type LaunchActions = Pick<
	AppState,
	| 'markLaunching'
	| 'clearLaunching'
	| 'launch'
	| 'closeApps'
	| 'getUninstallPreview'
	| 'uninstall'
>

export function createLaunchActions({
	set,
	get,
	client,
}: LaunchActionOptions): LaunchActions {
	const launchTimers = new Map<string, ReturnType<typeof setTimeout>>()

	return {
		markLaunching(id) {
			set(state =>
				state.launchingIds.includes(id)
					? state
					: { launchingIds: [...state.launchingIds, id] },
			)
		},
		clearLaunching(id) {
			const timer = launchTimers.get(id)
			if (timer) {
				clearTimeout(timer)
				launchTimers.delete(id)
			}
			set(state =>
				state.launchingIds.includes(id)
					? {
							launchingIds: state.launchingIds.filter(
								appId => appId !== id,
							),
						}
					: state,
			)
		},
		async launch(app) {
			set({ error: null })
			get().markLaunching(app.id)
			const existing = launchTimers.get(app.id)
			if (existing) clearTimeout(existing)
			launchTimers.set(
				app.id,
				setTimeout(
					() => get().clearLaunching(app.id),
					LAUNCH_CEILING_MS,
				),
			)
			try {
				await client.launchApp({ id: app.id })
			} catch (error) {
				get().clearLaunching(app.id)
				throw error
			}
		},
		async closeApps(ids) {
			set({ error: null })
			return client.closeApps(ids)
		},
		async getUninstallPreview(id) {
			return client.getUninstallPreview(id)
		},
		async uninstall(id) {
			set({ error: null })
			return client.uninstallApp(id)
		},
	}
}
