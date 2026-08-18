import { useRef } from 'react'
import { useModalDialog } from '../../../../shared/hooks/useModalDialog'
import { AdditionalInformation } from './AdditionalInformation'
import { AppIdentityHeader } from './AppIdentityHeader'
import { AppInformationCards } from './AppInformationCards'
import { DialogActions } from './DialogActions'
import type { AppInfoDialogProps } from './types'
import { useAppDetails } from '../../model/useAppDetails'
import { useAppInfoActions } from '../../model/useAppInfoActions'

export function AppInfoDialog({
	app,
	categories,
	appsClient,
	onClose,
}: AppInfoDialogProps) {
	const dialogRef = useRef<HTMLElement>(null)
	const closeRef = useRef<HTMLButtonElement>(null)
	useModalDialog({
		ref: dialogRef,
		initialFocusRef: closeRef,
		onDismiss: onClose,
	})
	const { details, error, isLoading, retry } = useAppDetails(
		app.id,
		appsClient,
	)
	const actions = useAppInfoActions({ app, details, appsClient })
	const hasKnownPackageInstallLocation =
		app.sourceKind === 'msix' && Boolean(app.installLocation?.trim())
	const hasPackageLaunchTarget = app.launchKind === 'app_user_model_id'
	return (
		<div
			className="motion-overlay fixed inset-0 z-400 grid place-items-center bg-[color-mix(in_oklab,var(--surface-canvas)_72%,transparent)] p-3 backdrop-blur-[3px] sm:p-5"
			onMouseDown={event => {
				if (event.currentTarget === event.target) onClose()
			}}
		>
			<section
				ref={dialogRef}
				role="dialog"
				aria-modal="true"
				aria-label={`${app.name} information`}
				className="motion-panel max-h-[calc(100vh-1.5rem)] w-full max-w-6xl overflow-y-auto rounded-3xl border border-(--border-neutral) bg-(--surface-panel) p-4 text-(--text-primary) shadow-(--shadow-dialog) sm:max-h-[calc(100vh-2.5rem)] sm:p-6"
			>
				<AppIdentityHeader
					app={app}
					categories={categories}
					details={details}
					closeRef={closeRef}
					onClose={onClose}
				/>
				<div className="mt-5 space-y-4">
					<DialogActions
						canOpenFolder={details?.canOpenFolder ?? false}
						isOpeningFolder={actions.isOpeningFolder}
						message={actions.message}
						onCopyPath={actions.copyPath}
						onCopyReport={actions.copyReport}
						onOpenFolder={actions.openFolder}
					/>
					{error ? (
						<div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-(--border-neutral) bg-(--surface-inset) px-4 py-3 text-sm text-(--text-muted)">
							<p role="alert">{error}</p>
							<button
								type="button"
								onClick={retry}
								className="min-h-10 rounded-lg px-3 text-(--accent-strong) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-(--accent-strong)"
							>
								Retry
							</button>
						</div>
					) : isLoading ? (
						<p className="text-sm text-(--text-muted)">
							Loading application details…
						</p>
					) : null}
					<AppInformationCards
						app={app}
						categories={categories}
						details={details}
						isLoading={isLoading}
					/>
					<AdditionalInformation
						details={details}
						isLoading={isLoading}
						hasKnownPackageInstallLocation={
							hasKnownPackageInstallLocation
						}
						hasPackageLaunchTarget={hasPackageLaunchTarget}
					/>
				</div>
			</section>
		</div>
	)
}
