import type { AppsClient } from '../../entities/app'
import type { AppState, GetAppState, SetAppState } from './types'

// Ceiling for the "launching" visual when the backend can't report real readiness
// (Store/UWP, shell hand-off). Cleared early by the launch://status event when available.
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

/**
 * Launch and uninstall.
 *
 * Actions reject; they do not also write `error`. `error` is the *background* failure channel —
 * work nobody is waiting on, which only `load()` produces — and App toasts it. An action already
 * has an owner for its message (`useAppFeedback` for launch/refresh/uninstall, `useSystemSettings`
 * for the maintenance scans), so writing it here too reported one failure twice, the second time
 * with no Retry.
 */
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
				setTimeout(() => get().clearLaunching(app.id), LAUNCH_CEILING_MS),
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
