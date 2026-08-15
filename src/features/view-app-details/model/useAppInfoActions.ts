import { useCallback, useState } from 'react'
import {
	type AppDetails,
	type AppInfo,
	type AppsClient,
	buildAppReport,
} from '../../../entities/app'
import { copyToClipboard } from '../../../shared/lib/clipboard'

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
