import { useDraggable } from '@dnd-kit/core'
import { CSS } from '@dnd-kit/utilities'
import { Grip } from 'lucide-react'
import { useCallback, useRef, useState } from 'react'
import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { isCatalogArtifact } from '../../lib/catalogArtifacts'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import { CardIcon } from './CardIcon'
import { CardLabel } from './CardLabel'
import { FavoriteButton } from './FavoriteButton'
import type { AppCardProps } from './types'

/**
 * How one application looks in the catalog. The card owns its own presentation — drag handle,
 * grip menu trigger, focus restoration, favorite toggle — and nothing about launching,
 * uninstalling, hiding or moving: those are features, injected through `renderActions`.
 */
export function AppCard({
	app,
	isFavorite,
	launching,
	onToggleFavorite,
	onLaunch,
	isAuxiliary = false,
	isDragPreviewActive = false,
	renderActions,
}: AppCardProps) {
	const [menuOpen, setMenuOpen] = useState(false)
	const gripRef = useRef<HTMLButtonElement | null>(null)
	// Return focus to the grip trigger when the menu closes (keyboard users keep their place).
	const closeMenu = useCallback(() => {
		setMenuOpen(false)
		gripRef.current?.focus()
	}, [])
	const spotlight = useSpotlight()
	const artifact = isCatalogArtifact(app)
	const draggable = useDraggable({
		id: `app:${app.id}`,
		data: { type: 'app', appId: app.id, category: app.category },
		disabled: artifact,
	})
	return (
		<article
			ref={draggable.setNodeRef}
			data-menu-open={menuOpen || undefined}
			onPointerMove={spotlight.onPointerMove}
			onPointerEnter={spotlight.onPointerEnter}
			onPointerLeave={spotlight.onPointerLeave}
			style={{
				transform: draggable.isDragging
					? undefined
					: CSS.Translate.toString(draggable.transform),
			}}
			data-launching={launching || undefined}
			className={`app-card app-card-glass cv-card group relative min-h-34 rounded-[1.15rem] border border-white/85 transition-[transform,border-color,box-shadow,opacity] duration-200 ease-out hover:-translate-y-0.5 focus-within:border-violet-400/45 ${menuOpen ? 'z-100' : ''} ${draggable.isDragging && isDragPreviewActive ? 'z-40 opacity-60' : ''}`}
		>
			<SpotlightLayer size={110} />
			<button
				type='button'
				onClick={() => {
					if (launching) return
					void onLaunch(app)
				}}
				aria-label={`Launch ${app.name}`}
				aria-busy={launching}
				disabled={launching}
				title={launching ? 'Launching…' : app.name}
				className='relative z-1 flex min-h-34 w-full flex-col items-center justify-center gap-3 px-4 py-4 text-center focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-violet-500 disabled:cursor-progress'
			>
				<CardIcon iconBase64={app.iconBase64} launching={launching} />
				<CardLabel
					name={app.name}
					version={app.version}
					launching={launching}
				/>
			</button>
			<button
				type='button'
				ref={node => {
					draggable.setActivatorNodeRef(node)
					gripRef.current = node
				}}
				{...draggable.listeners}
				{...draggable.attributes}
				aria-label={`Manage ${app.name}`}
				aria-expanded={menuOpen}
				aria-haspopup='menu'
				onClick={event => {
					event.stopPropagation()
					setMenuOpen(value => !value)
				}}
				className='absolute left-2 top-2 z-2 grid size-8 cursor-grab place-items-center rounded-lg border border-white/85 bg-white/72 text-slate-500 opacity-75 shadow-sm transition hover:text-violet-700 hover:opacity-100 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-violet-500 active:cursor-grabbing'
			>
				<Grip size={16} aria-hidden='true' />
			</button>
			{!isAuxiliary && !artifact && (
				<FavoriteButton
					appName={app.name}
					isFavorite={isFavorite}
					onToggle={() => onToggleFavorite(app.id)}
				/>
			)}
			{menuOpen && renderActions({ close: closeMenu, anchorRef: gripRef })}
		</article>
	)
}
