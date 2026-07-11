import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'

export type UpdateCheckStatus = 'idle' | 'checking' | 'current' | 'available' | 'error'

export interface UpdaterState {
	update: { version: string; notes: string | null } | null
	installing: boolean
	progress: number | null
	status: UpdateCheckStatus
	checkNow(): Promise<void>
	install(): Promise<void>
	dismiss(): void
}

interface Options {
	/** Check automatically on mount. Off for the manual Settings button. */
	autoCheck?: boolean
}

const DISMISSED_UPDATE_KEY = 'windows-apps.dismissed-update-version'

function dismissedVersion(): string | null {
	try {
		return globalThis.localStorage?.getItem(DISMISSED_UPDATE_KEY) ?? null
	} catch {
		return null
	}
}

function rememberDismissedVersion(version: string) {
	try {
		globalThis.localStorage?.setItem(DISMISSED_UPDATE_KEY, version)
	} catch {
		// Storage can be unavailable in restricted environments; dismissal still hides
		// the current in-memory prompt for this session.
	}
}

function shouldShowUpdate(found: Update | null, ignoreDismissed: boolean): boolean {
	if (!found) return false
	return ignoreDismissed || dismissedVersion() !== found.version
}

/**
 * Checks for an application update (GitHub Releases endpoint). Silent when there is no
 * update, no network, or when not running inside Tauri (dev browser / tests). `install()`
 * downloads with progress, then relaunches into the new version. `checkNow()` runs an
 * on-demand check and reports `status` for a manual "Check for updates" control.
 */
export function useUpdater(options?: Options): UpdaterState {
	const autoCheck = options?.autoCheck ?? true
	const [available, setAvailable] = useState<Update | null>(null)
	const [installing, setInstalling] = useState(false)
	const [progress, setProgress] = useState<number | null>(null)
	const [status, setStatus] = useState<UpdateCheckStatus>('idle')

	const checkNow = useCallback(async () => {
		setStatus('checking')
		try {
			const found = await check()
			setAvailable(shouldShowUpdate(found, true) ? found : null)
			setStatus(found ? 'available' : 'current')
		} catch {
			setStatus('error')
		}
	}, [])

	useEffect(() => {
		if (!autoCheck) return
		let active = true
		void (async () => {
			try {
				const found = await check()
				if (active && shouldShowUpdate(found, false)) {
					setAvailable(found)
					setStatus('available')
				}
			} catch {
				// Not in Tauri, offline, or no published release yet — nothing to surface.
			}
		})()
		return () => {
			active = false
		}
	}, [autoCheck])

	const install = useCallback(async () => {
		if (!available || installing) return
		setInstalling(true)
		setProgress(0)
		try {
			let total = 0
			let downloaded = 0
			await available.downloadAndInstall(event => {
				switch (event.event) {
					case 'Started':
						total = event.data.contentLength ?? 0
						setProgress(0)
						break
					case 'Progress':
						downloaded += event.data.chunkLength
						setProgress(
							total
								? Math.min(100, Math.round((downloaded / total) * 100))
								: null,
						)
						break
					case 'Finished':
						setProgress(100)
						break
				}
			})
			await relaunch()
		} catch (error) {
			setInstalling(false)
			setProgress(null)
			const reason = error instanceof Error ? error.message : String(error)
			toast.error(`Update failed: ${reason}`)
		}
	}, [available, installing])

	const dismiss = useCallback(() => {
		if (available) rememberDismissedVersion(available.version)
		setAvailable(null)
	}, [available])

	return {
		update: available
			? { version: available.version, notes: available.body ?? null }
			: null,
		installing,
		progress,
		status,
		checkNow,
		install,
		dismiss,
	}
}
