import { UpdateDialog, type useUpdater } from '../../features/update-app'
import type { StaleCopyInfo, SystemClient } from '../../entities/system'
import { GlobalActivityBar } from './GlobalActivityBar'
import { PreferencesNotSavedBanner } from './PreferencesNotSavedBanner'
import { StaleCopyBanner } from '../../features/stale-copy'
import { TitleBar } from './TitleBar'

interface AppShellChromeProps {
	activityActive: boolean
	activityLabel: string
	preferencesPersisted: boolean
	staleCopy: StaleCopyInfo | null
	systemClient: Pick<
		SystemClient,
		'openGithub' | 'openInstalledCopy' | 'openRelease'
	>
	updater: ReturnType<typeof useUpdater>
	onDismissStaleCopy: () => void
}

/**
 * The window chrome and the app-level status surfaces: title bar, the banners that report a
 * degraded install or unsaved preferences, the update flow, and the global activity indicator.
 *
 * Grouped because they share one responsibility — telling the user about the state of the
 * application itself rather than the catalog — and because they render above and outside the
 * catalog workspace regardless of which view is active.
 */
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
