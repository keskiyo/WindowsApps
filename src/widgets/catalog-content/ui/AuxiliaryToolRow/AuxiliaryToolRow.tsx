import { useDraggable } from '@dnd-kit/core'
import { CSS } from '@dnd-kit/utilities'
import { EllipsisVertical } from 'lucide-react'
import { memo, useCallback, useRef, useState } from 'react'
import { useIsLaunching } from '../../../../features/launch-app'
import { AppActionsMenu } from '../../../../features/app-actions'
import { CardIcon } from '../../../../entities/app'
import type { AuxiliaryToolRowProps } from './types'

function AuxiliaryToolRowComponent({
	app,
	categories,
	categoryOrder,
	onLaunch,
	onMove,
	onInfo,
	onUninstall,
	onRestore,
	onDemote,
}: AuxiliaryToolRowProps) {
	const [menuOpen, setMenuOpen] = useState(false)
	const launching = useIsLaunching(app.id)
	const manageRef = useRef<HTMLButtonElement | null>(null)
	const closeMenu = useCallback(() => {
		setMenuOpen(false)
		manageRef.current?.focus()
	}, [])
	const draggable = useDraggable({
		id: `app:${app.id}`,
		data: { type: 'app', appId: app.id, category: app.category },
	})
	const metadata = [app.publisher?.trim(), app.version?.trim()]
		.filter(Boolean)
		.join(' · ')

	return (
		<article
			ref={draggable.setNodeRef}
			data-menu-open={menuOpen || undefined}
			data-launching={launching || undefined}
			style={{ transform: CSS.Translate.toString(draggable.transform) }}
			className={`app-card app-card-glass group relative grid min-h-18 min-w-0 grid-cols-[minmax(0,1fr)_auto] items-stretch rounded-xl border border-white/85 transition-[transform,border-color,box-shadow,opacity] duration-200 ease-out focus-within:border-violet-400/45 hover:-translate-y-0.5 ${menuOpen ? 'z-100' : ''} ${draggable.isDragging ? 'z-40 opacity-60' : ''}`}
		>
			<button
				type="button"
				onClick={() => {
					if (!launching) void onLaunch(app)
				}}
				aria-label={`Launch ${app.name}`}
				aria-busy={launching}
				disabled={launching}
				title={launching ? 'Launching…' : app.name}
				className="relative z-1 grid min-w-0 grid-cols-[auto_minmax(0,1fr)] items-center gap-3 rounded-l-xl px-3 py-2.5 text-left focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-violet-500 disabled:cursor-progress"
			>
				<CardIcon iconBase64={app.iconBase64} launching={launching} />
				<span className="min-w-0">
					<span className="block truncate text-sm font-semibold text-slate-800">
						{app.name}
					</span>
					{metadata && (
						<span
							className="mt-0.5 block truncate text-xs text-slate-500"
							title={metadata}
						>
							{metadata}
						</span>
					)}
				</span>
			</button>
			<button
				type="button"
				ref={node => {
					draggable.setActivatorNodeRef(node)
					manageRef.current = node
				}}
				{...draggable.listeners}
				{...draggable.attributes}
				aria-label={`Manage ${app.name}`}
				aria-expanded={menuOpen}
				aria-haspopup="menu"
				onClick={event => {
					event.stopPropagation()
					setMenuOpen(value => !value)
				}}
				className="relative z-2 mr-2 grid size-8 cursor-grab place-items-center self-center rounded-lg border border-white/85 bg-white/72 text-slate-500 opacity-75 shadow-sm transition hover:text-violet-700 hover:opacity-100 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-violet-500 active:cursor-grabbing"
			>
				<EllipsisVertical size={16} aria-hidden="true" />
			</button>
			{menuOpen && (
				<AppActionsMenu
					app={app}
					categories={categories}
					categoryOrder={categoryOrder}
					onClose={closeMenu}
					onMove={onMove}
					onInfo={onInfo}
					onUninstall={onUninstall}
					isHidden
					onHide={() => undefined}
					onRestore={onRestore}
					onDemote={onDemote}
					anchorRef={manageRef}
				/>
			)}
		</article>
	)
}

export const AuxiliaryToolRow = memo(AuxiliaryToolRowComponent)
