import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { useCallback, useEffect, useRef, useState } from 'react'

export type UpdateCheckStatus = 'idle' | 'checking' | 'current' | 'available' | 'error'
export type UpdateInstallPhase =
	| 'idle'
	| 'downloading'
	| 'verifying'
	| 'installing'
	| 'restarting'
	| 'failed'

export interface AvailableUpdate {
	version: string
	notes: string | null
	date: string | null
	packageSize: number | null
	releaseUrl: string | null
}

export interface UpdaterState {
	update: AvailableUpdate | null
	installing: boolean
	progress: number | null
	downloadedBytes: number
	totalBytes: number | null
	phase: UpdateInstallPhase
	error: string | null
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

function positiveNumber(value: unknown): number | null {
	return typeof value === 'number' && Number.isFinite(value) && value > 0
		? value
		: null
}

function httpUrl(value: unknown): string | null {
	if (typeof value !== 'string') return null
	try {
		const url = new URL(value)
		return url.protocol === 'https:' ? url.toString() : null
	} catch {
		return null
	}
}

function updateErrorMessage(error: unknown): string {
	const reason = error instanceof Error ? error.message : String(error)
	const normalized = reason.toLowerCase()
	if (normalized.includes('404') || normalized.includes('not found')) {
		return 'The update package is unavailable. Try again later or download it from GitHub.'
	}
	if (
		normalized.includes('download') ||
		normalized.includes('network') ||
		normalized.includes('request')
	) {
		return 'Could not download the update. Check your connection and try again.'
	}
	if (normalized.includes('signature') || normalized.includes('verify')) {
		return 'Update verification failed. The package was not installed.'
	}
	return 'The update could not be installed. Try again or download it manually.'
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
	const [phase, setPhase] = useState<UpdateInstallPhase>('idle')
	const [progress, setProgress] = useState<number | null>(null)
	const [downloadedBytes, setDownloadedBytes] = useState(0)
	const [totalBytes, setTotalBytes] = useState<number | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [status, setStatus] = useState<UpdateCheckStatus>('idle')
	const checkPromiseRef = useRef<Promise<Update | null> | null>(null)
	const installInFlightRef = useRef(false)

	const requestCheck = useCallback(() => {
		if (checkPromiseRef.current) return checkPromiseRef.current
		const request = check().finally(() => {
			if (checkPromiseRef.current === request) checkPromiseRef.current = null
		})
		checkPromiseRef.current = request
		return request
	}, [])

	const checkNow = useCallback(async () => {
		setStatus('checking')
		try {
			const found = await requestCheck()
			setAvailable(shouldShowUpdate(found, true) ? found : null)
			setStatus(found ? 'available' : 'current')
		} catch {
			setStatus('error')
		}
	}, [requestCheck])

	useEffect(() => {
		if (!autoCheck) return
		let active = true
		void (async () => {
			try {
				const found = await requestCheck()
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
	}, [autoCheck, requestCheck])

	const install = useCallback(async () => {
		if (
			installInFlightRef.current ||
			!available ||
			['downloading', 'verifying', 'installing', 'restarting'].includes(phase)
		)
			return
		installInFlightRef.current = true
		setError(null)
		setPhase('downloading')
		setProgress(0)
		setDownloadedBytes(0)
		setTotalBytes(null)
		try {
			let total = 0
			let downloaded = 0
			await available.download(event => {
				switch (event.event) {
					case 'Started':
						total = event.data.contentLength ?? 0
						setPhase('downloading')
						setTotalBytes(total || null)
						setProgress(0)
						break
					case 'Progress':
						downloaded += event.data.chunkLength
						setDownloadedBytes(downloaded)
						setProgress(
							total
								? Math.min(100, Math.round((downloaded / total) * 100))
								: null,
						)
						break
					case 'Finished':
						break
				}
			})
			setPhase('verifying')
			setProgress(100)
			await Promise.resolve()
			setPhase('installing')
			await available.install()
			setPhase('restarting')
			await relaunch()
		} catch (error) {
			console.error('Update installation failed', error)
			setPhase('failed')
			setProgress(null)
			setError(updateErrorMessage(error))
		} finally {
			installInFlightRef.current = false
		}
	}, [available, phase])

	const dismiss = useCallback(() => {
		if (available) rememberDismissedVersion(available.version)
		setAvailable(null)
	}, [available])

	return {
		update: available
			? {
					version: available.version,
					notes: available.body ?? null,
					date: available.date ?? null,
					packageSize: positiveNumber(available.rawJson.packageSize),
					releaseUrl: httpUrl(available.rawJson.releaseUrl),
				}
			: null,
		installing: ['downloading', 'verifying', 'installing', 'restarting'].includes(
			phase,
		),
		progress,
		downloadedBytes,
		totalBytes,
		phase,
		error,
		status,
		checkNow,
		install,
		dismiss,
	}
}
