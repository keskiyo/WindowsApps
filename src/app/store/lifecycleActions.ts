import type { AppsClient } from '../../entities/app'
import type { AppState, GetAppState, SetAppState } from './types'

interface LifecycleOptions {
	set: SetAppState
	get: GetAppState
	client: AppsClient
}

/**
 * Catalog subscriptions plus the first load, reference-counted so several mounts share one
 * registration and the last one to leave detaches it.
 */
export function createLifecycleActions({
	set,
	get,
	client,
}: LifecycleOptions): Pick<AppState, 'initialize'> {
	let initializationPromise: Promise<() => void> | null = null
	let initializationDispose: (() => void) | null = null
	let initializationUsers = 0

	function releaseInitialization() {
		initializationUsers = Math.max(0, initializationUsers - 1)
		if (initializationUsers > 0) return
		initializationDispose?.()
		initializationDispose = null
		initializationPromise = null
	}

	return {
		async initialize() {
			initializationUsers += 1
			if (!initializationPromise) {
				initializationPromise = (async () => {
					const disposers: Array<() => void> = []
					const subscribe = async <T>(
						registration:
							| ((handler: (value: T) => void) => Promise<() => void>)
							| undefined,
						handler: (value: T) => void,
					) => {
						if (registration)
							disposers.push(await registration(handler))
					}
					// Registration is all-or-nothing. Each listener was awaited in turn, so a
					// rejection partway through left the earlier ones attached with no owner
					// to detach them, and the rejected promise was cached — every later
					// initialize() returned that same failure without ever retrying. On
					// failure the accumulated disposers run and the cached state is cleared,
					// so a transient bridge error is recoverable.
					try {
						await subscribe(client.onCatalogDelta, get().applyDelta)
						await subscribe(client.onCatalogPatches, get().applyPatches)
						await subscribe(client.onCatalogChanged, summary =>
							set({ catalogChange: summary }),
						)
						disposers.push(await client.onAppsUpdated(get().replaceApps))
						disposers.push(
							await client.onScanProgress(scanProgress =>
								set({ scanProgress }),
							),
						)
						await subscribe(client.onLaunchStatus, status =>
							get().clearLaunching(status.id),
						)
					} catch (error) {
						disposers.splice(0).forEach(dispose => {
							try {
								dispose()
							} catch {
								// One listener that refuses to detach must not strand the rest.
							}
						})
						initializationUsers = Math.max(0, initializationUsers - 1)
						initializationDispose = null
						initializationPromise = null
						throw error
					}
					await get().load()
					if (get().hasCache) await client.startBackgroundSync?.()
					initializationDispose = () =>
						disposers.splice(0).forEach(dispose => dispose())
					return releaseInitialization
				})()
			}
			return initializationPromise
		},
	}
}
