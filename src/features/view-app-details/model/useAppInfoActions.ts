import { useCallback, useState } from 'react'
import { buildAppReport } from '../../../lib/appMetadata'
import type { AppDetails, AppInfo, AppsClient } from '../../../types'

async function copyToClipboard(value: string): Promise<void> {
	if (navigator.clipboard?.writeText) {
		await navigator.clipboard.writeText(value)
		return
	}
	const element = document.createElement('textarea')
	element.value = value
	element.setAttribute('readonly', '')
	element.className = 'fixed -left-full top-0 opacity-0'
	document.body.append(element)
	element.select()
	const copied = document.execCommand?.('copy')
	element.remove()
	if (!copied) throw new Error('Clipboard is unavailable')
}

interface AppInfoActionOptions {
	app: AppInfo
	details: AppDetails | null
	appsClient: Pick<AppsClient, 'openAppFolder'>
}

export function useAppInfoActions({
	app,
	details,
	appsClient,
}: AppInfoActionOptions) {
	const [message, setMessage] = useState<string | null>(null)
	const [isOpeningFolder, setIsOpeningFolder] = useState(false)
	const copyReport = useCallback(async () => {
		try {
			await copyToClipboard(buildAppReport(app, details))
			setMessage('Report copied.')
		} catch {
			setMessage('Could not copy the report.')
		}
	}, [app, details])
	const copyPath = useCallback(async () => {
		try {
			await copyToClipboard(app.path)
			setMessage('Path copied.')
		} catch {
			setMessage('Could not copy the path.')
		}
	}, [app.path])
	const openFolder = useCallback(async () => {
		setIsOpeningFolder(true)
		try {
			await appsClient.openAppFolder(app.id)
			setMessage(null)
		} catch {
			setMessage('Could not open the application folder.')
		} finally {
			setIsOpeningFolder(false)
		}
	}, [app.id, appsClient])
	return {
		copyPath,
		copyReport,
		isOpeningFolder,
		message,
		openFolder,
	}
}
