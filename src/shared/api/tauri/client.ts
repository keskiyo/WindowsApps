import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { AppClientError, toAppClientError } from './errors'

type TauriGlobal = typeof globalThis & {
	__TAURI_INTERNALS__?: unknown
}

/**
 * The generic Tauri transport. It knows commands, events and error normalization, and nothing
 * about App, Category or System — the typed clients that do live in each entity's `api` segment.
 */
export function isTauriRuntime(): boolean {
	return Boolean((globalThis as TauriGlobal).__TAURI_INTERNALS__)
}

export async function invokeTauri<T>(
	command: string,
	args?: Record<string, unknown>,
): Promise<T> {
	try {
		return await invoke<T>(command, args)
	} catch (error) {
		throw toAppClientError(error)
	}
}

export async function invokeIfTauri<T>(
	command: string,
	args?: Record<string, unknown>,
): Promise<T> {
	if (!isTauriRuntime())
		throw new AppClientError(
			'DESKTOP_RUNTIME_UNAVAILABLE',
			'This action is available only in the desktop app.',
		)
	return invokeTauri<T>(command, args)
}

export async function listenIfTauri<T>(
	event: string,
	handler: (payload: T) => void,
): Promise<() => void> {
	if (!isTauriRuntime()) return () => undefined
	return listen<T>(event, ({ payload }) => handler(payload))
}
