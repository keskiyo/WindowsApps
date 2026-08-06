import { EllipsisVertical } from 'lucide-react'
import { useCallback, useRef, useState } from 'react'
import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { isCatalogArtifact } from '../../lib/catalogArtifacts'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import { CardIcon } from './CardIcon'
import { CardLabel } from './CardLabel'
import { FavoriteButton } from './FavoriteButton'
import type { AppCardProps } from './types'

export function AppCard({
	app,
	isFavorite,
	launching,
	onToggleFavorite,
	onLaunch,
	isAuxiliary = false,
	renderActions,
}: AppCardProps) {
	const [menuOpen, setMenuOpen] = useState(false)
	const menuTriggerRef = useRef<HTMLButtonElement | null>(null)
	const closeMenu = useCallback(() => {
		setMenuOpen(false)
		menuTriggerRef.current?.focus()
	}, [])
	const spotlight = useSpotlight()
	const artifact = isCatalogArtifact(app)
	return (
		<article
			data-menu-open={menuOpen || undefined}
			onPointerMove={spotlight.onPointerMove}
			onPointerEnter={spotlight.onPointerEnter}
			onPointerLeave={spotlight.onPointerLeave}
			data-launching={launching || undefined}
			className={`app-card app-card-tile app-card-glass cv-card group relative rounded-[1.15rem] border border-white/85 transition-[transform,border-color,box-shadow,opacity] duration-200 ease-out focus-within:border-violet-400/45 hover:-translate-y-0.5 ${menuOpen ? 'z-100' : ''}`}
		>
			<SpotlightLayer size={110} />
			<button
				type="button"
				onClick={() => {
					if (launching) return
					void onLaunch(app)
				}}
				aria-label={`Launch ${app.name}`}
				aria-busy={launching}
				disabled={launching}
				title={launching ? 'Launching…' : app.name}
				className="relative z-1 flex size-full flex-col items-center justify-center gap-3 px-3 py-4 text-center focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-violet-500 disabled:cursor-progress"
			>
				<CardIcon iconBase64={app.iconBase64} launching={launching} />
				<CardLabel
					name={app.name}
					version={app.version}
					launching={launching}
				/>
			</button>
			<button
				type="button"
				ref={menuTriggerRef}
				aria-label={`Manage ${app.name}`}
				aria-expanded={menuOpen}
				aria-haspopup="menu"
				onClick={event => {
					event.stopPropagation()
					setMenuOpen(value => !value)
				}}
				className="absolute top-2 left-2 z-2 grid size-8 place-items-center rounded-lg border border-white/85 bg-white/72 text-slate-500 opacity-75 shadow-sm transition hover:text-violet-700 hover:opacity-100 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-violet-500"
			>
				<EllipsisVertical size={16} aria-hidden="true" />
			</button>
			{!isAuxiliary && !artifact && (
				<FavoriteButton
					appName={app.name}
					isFavorite={isFavorite}
					onToggle={() => onToggleFavorite(app.id)}
				/>
			)}
			{menuOpen &&
				renderActions({ close: closeMenu, anchorRef: menuTriggerRef })}
		</article>
	)
}
