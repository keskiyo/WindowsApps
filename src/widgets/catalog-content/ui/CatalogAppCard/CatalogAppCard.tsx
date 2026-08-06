import { memo } from 'react'
import { AppCard } from '../../../../entities/app'
import { AppActionsMenu } from '../../../../features/app-actions'
import { useIsLaunching } from '../../../../features/launch-app'
import type { CatalogAppCardProps } from './types'

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
}: CatalogAppCardProps) {
	const launching = useIsLaunching(app.id)
	return (
		<AppCard
			app={app}
			isFavorite={isFavorite}
			launching={launching}
			onToggleFavorite={onToggleFavorite}
			onLaunch={onLaunch}
			isAuxiliary={isAuxiliary}
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

export const CatalogAppCard = memo(CatalogAppCardComponent)
