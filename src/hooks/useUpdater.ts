import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'

export interface UpdaterState {
	update: { version: string; notes: string | null } | null
	installing: boolean
	progress: number | null
	install(): Promise<void>
	dismiss(): void
}

/**
 * Checks for an application update once on mount (GitHub Releases endpoint). Silent when
 * there is no update, no network, or when not running inside Tauri (dev browser / tests).
 * `install()` downloads with progress, then relaunches into the new version.
 */
export function useUpdater(): UpdaterState {
	const [available, setAvailable] = useState<Update | null>(null)
	const [installing, setInstalling] = useState(false)
	const [progress, setProgress] = useState<number | null>(null)

	useEffect(() => {
		let active = true
		void (async () => {
			try {
				const found = await check()
				if (active && found) setAvailable(found)
			} catch {
				// Not in Tauri, offline, or no published release yet — nothing to surface.
			}
		})()
		return () => {
			active = false
		}
	}, [])

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

	const dismiss = useCallback(() => setAvailable(null), [])

	return {
		update: available
			? { version: available.version, notes: available.body ?? null }
			: null,
		installing,
		progress,
		install,
		dismiss,
	}
}
