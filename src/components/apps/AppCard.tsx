import { useDraggable } from '@dnd-kit/core'
import { CSS } from '@dnd-kit/utilities'
import { AppWindow, Grip, Star } from 'lucide-react'
import { memo, useCallback, useState } from 'react'
import { useSpotlight } from '../../hooks/useSpotlight'
import type { AppCategory, AppInfo, CategoryDefinition } from '../../types'
import { SpotlightLayer } from '../shared/SpotlightLayer'
import { AppActionsMenu } from './AppActionsMenu'

interface AppCardProps {
	app: AppInfo
	isFavorite: boolean
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	onMove(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	isHidden?: boolean
	onHide(id: string): void
	onRestore(id: string): void
}

function AppCardComponent({
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
	onHide,
	onRestore,
}: AppCardProps) {
	const [menuOpen, setMenuOpen] = useState(false)
	const closeMenu = useCallback(() => setMenuOpen(false), [])
	const spotlight = useSpotlight()
	const draggable = useDraggable({
		id: `app:${app.id}`,
		data: { type: 'app', appId: app.id, category: app.category },
	})
	return (
		<article
			ref={draggable.setNodeRef}
			data-menu-open={menuOpen || undefined}
			onPointerMove={spotlight.onPointerMove}
			onPointerEnter={spotlight.onPointerEnter}
			onPointerLeave={spotlight.onPointerLeave}
			style={{ transform: CSS.Translate.toString(draggable.transform) }}
			className={`app-card app-card-glass group relative min-h-34 rounded-[1.15rem] border border-white/85 transition-[transform,border-color,box-shadow,opacity] duration-200 ease-out hover:-translate-y-0.5 focus-within:border-violet-400/45 ${menuOpen ? 'z-100' : ''} ${draggable.isDragging ? 'z-40 opacity-60' : ''}`}
		>
			<SpotlightLayer size={110} />
			<button
				type='button'
				onClick={() => void onLaunch(app)}
				aria-label={`Launch ${app.name}`}
				title={app.path}
				className='relative z-1 flex min-h-34 w-full flex-col items-center justify-center gap-3 px-4 py-4 text-center focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-violet-500'
			>
				<span className='grid size-13 place-items-center rounded-xl bg-white/52 shadow-[inset_1px_1px_3px_rgba(111,124,146,.13),inset_-2px_-2px_5px_rgba(255,255,255,.85)] ring-1 ring-inset ring-white/80'>
					{app.iconBase64 ? (
						<img
							src={app.iconBase64}
							alt=''
							className='size-9.5 object-contain'
							draggable={false}
						/>
					) : (
						<AppWindow
							size={27}
							className='text-slate-500 transition-colors group-hover:text-violet-600'
							aria-hidden='true'
						/>
					)}
				</span>
				<span className='w-full truncate text-sm font-semibold text-slate-700 group-hover:text-slate-900'>
					{app.name}
				</span>
			</button>
			<button
				type='button'
				ref={draggable.setActivatorNodeRef}
				{...draggable.listeners}
				{...draggable.attributes}
				aria-label={`Manage ${app.name}`}
				aria-expanded={menuOpen}
				onClick={event => {
					event.stopPropagation()
					setMenuOpen(value => !value)
				}}
				className='absolute left-2 top-2 z-2 grid size-8 cursor-grab place-items-center rounded-lg border border-white/85 bg-white/72 text-slate-500 opacity-75 shadow-sm transition hover:text-violet-700 hover:opacity-100 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-violet-500 active:cursor-grabbing'
			>
				<Grip size={16} aria-hidden='true' />
			</button>
			<button
				type='button'
				aria-label={`${isFavorite ? 'Remove' : 'Add'} ${app.name} ${isFavorite ? 'from' : 'to'} favorites`}
				aria-pressed={isFavorite}
				onClick={event => {
					event.stopPropagation()
					onToggleFavorite(app.id)
				}}
				className={`absolute right-2 top-2 z-2 grid size-8 place-items-center rounded-lg border transition focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-amber-500 ${isFavorite ? 'border-amber-400/35 bg-amber-100/85 text-amber-600 opacity-100' : 'border-white/85 bg-white/72 text-slate-400 opacity-0 hover:text-amber-600 group-hover:opacity-100 group-focus-within:opacity-100'}`}
			>
				<Star
					size={16}
					fill={isFavorite ? 'currentColor' : 'none'}
					aria-hidden='true'
				/>
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
					isHidden={isHidden}
					onHide={onHide}
					onRestore={onRestore}
				/>
			)}
		</article>
	)
}

// Memoized so background icon patches re-render only the changed cards, not the whole
// grid. All callback/array props from the parents are stable (store actions / useCallback).
export const AppCard = memo(AppCardComponent)
