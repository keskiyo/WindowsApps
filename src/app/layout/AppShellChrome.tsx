import { UpdateDialog } from '../../features/update-app'
import { GlobalActivityBar } from './GlobalActivityBar'
import { PreferencesNotSavedBanner } from './PreferencesNotSavedBanner'
import { StaleCopyBanner } from '../../features/stale-copy'
import { TitleBar } from './TitleBar'
import type { AppShellChromeProps } from '../types'

export function AppShellChrome({
	activityActive,
	activityLabel,
	preferencesPersisted,
	staleCopy,
	systemClient,
	updater,
	onDismissStaleCopy,
}: AppShellChromeProps) {
	return (
		<>
			<TitleBar />
			{staleCopy && (
				<StaleCopyBanner
					installedVersion={staleCopy.installedVersion}
					installLocation={staleCopy.installLocation}
					onOpenInstalled={() =>
						systemClient.openInstalledCopy?.() ?? Promise.resolve()
					}
					onDismiss={onDismissStaleCopy}
				/>
			)}
			{!preferencesPersisted && <PreferencesNotSavedBanner />}
			{updater.update && (
				<UpdateDialog
					version={updater.update.version}
					date={updater.update.date}
					packageSize={updater.update.packageSize}
					releaseUrl={updater.update.releaseUrl}
					notes={updater.update.notes}
					installing={updater.installing}
					progress={updater.progress}
					downloadedBytes={updater.downloadedBytes}
					totalBytes={updater.totalBytes}
					phase={updater.phase}
					error={updater.error}
					onInstall={() => void updater.install()}
					onDismiss={updater.dismiss}
					onOpenRelease={() =>
						void (
							systemClient.openRelease?.(
								updater.update?.version ?? '',
							) ?? systemClient.openGithub()
						)
					}
				/>
			)}
			<GlobalActivityBar active={activityActive} label={activityLabel} />
		</>
	)
}
