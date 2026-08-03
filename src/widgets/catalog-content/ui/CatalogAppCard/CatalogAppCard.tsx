import { memo } from 'react'
import { AppCard } from '../../../../entities/app'
import { AppActionsMenu } from '../../../../features/app-actions'
import { useIsLaunching } from '../../../../features/launch-app'
import type { CatalogAppCardProps } from './types'

/**
 * The catalog's card: the App entity's presentation wired to the features that act on it.
 * The entity stays free of launch, uninstall, hide and move so it can be rendered anywhere.
 */
function CatalogAppCardComponent({
	app,
	isFavorite,
	categories,
	categoryOrder,
	onToggleFavorite,
	onLaunch,
	onMove,
	onInfo,
	onUninstall,
	isHidden = false,
	isAuxiliary = false,
	onHide,
	onRestore,
	onDemote,
	isDragPreviewActive = false,
}: CatalogAppCardProps) {
	// Per-card subscription: a launch must not re-render the whole grid.
	const launching = useIsLaunching(app.id)
	return (
		<AppCard
			app={app}
			isFavorite={isFavorite}
			launching={launching}
			onToggleFavorite={onToggleFavorite}
			onLaunch={onLaunch}
			isAuxiliary={isAuxiliary}
			isDragPreviewActive={isDragPreviewActive}
			renderActions={({ close, anchorRef }) => (
				<AppActionsMenu
					app={app}
					categories={categories}
					categoryOrder={categoryOrder}
					onClose={close}
					onMove={onMove}
					onInfo={onInfo}
					onUninstall={onUninstall}
					isHidden={isHidden}
					isUserPromoted={app.userPromoted}
					onHide={onHide}
					onRestore={onRestore}
					onDemote={onDemote}
					anchorRef={anchorRef}
				/>
			)}
		/>
	)
}

// Memoized so background icon patches re-render only the changed cards, not the whole
// grid. All callback/array props from the parents are stable (store actions / useCallback).
export const CatalogAppCard = memo(CatalogAppCardComponent)
