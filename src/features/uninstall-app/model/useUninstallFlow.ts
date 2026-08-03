import { useCallback, useEffect, useState } from 'react'
import { toAppClientError } from '../../../shared/api/tauri/errors'
import type { AppInfo, UninstallPreview } from '../../../entities/app'

/**
 * Owns the uninstall confirmation: which app is pending, and the preview fetched for it.
 *
 * The preview request is guarded against stale results — selecting a second app before the
 * first response lands must not show the first app's mechanism next to the second app's name.
 */
export function useUninstallFlow(
	getUninstallPreview: (id: string) => Promise<UninstallPreview>,
) {
	const [app, setApp] = useState<AppInfo | null>(null)
	const [preview, setPreview] = useState<UninstallPreview | null>(null)
	const [isPreviewLoading, setPreviewLoading] = useState(false)
	const [previewError, setPreviewError] = useState<string | null>(null)

	useEffect(() => {
		if (!app) return
		let active = true
		setPreview(null)
		setPreviewError(null)
		setPreviewLoading(true)
		void getUninstallPreview(app.id)
			.then(value => {
				if (active) setPreview(value)
			})
			.catch(error => {
				if (active) setPreviewError(toAppClientError(error).message)
			})
			.finally(() => {
				if (active) setPreviewLoading(false)
			})
		return () => {
			active = false
		}
	}, [getUninstallPreview, app])

	const close = useCallback(() => {
		setApp(null)
		setPreview(null)
		setPreviewError(null)
	}, [])

	return { app, close, isPreviewLoading, preview, previewError, select: setApp }
}
