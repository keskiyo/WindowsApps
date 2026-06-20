import { useDraggable } from '@dnd-kit/core'
import { CSS } from '@dnd-kit/utilities'
import { AppWindow, Grip, Star } from 'lucide-react'
import { useCallback, useState } from 'react'
import type { AppCategory, AppInfo, CategoryDefinition } from '../../types'
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

export function AppCard({
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
	const draggable = useDraggable({
		id: `app:${app.id}`,
		data: { type: 'app', appId: app.id, category: app.category },
	})
	return (
		<article
			ref={draggable.setNodeRef}
			data-menu-open={menuOpen || undefined}
			style={{ transform: CSS.Translate.toString(draggable.transform) }}
			className={`app-card group relative min-h-34 rounded-2xl border border-white/[0.07] bg-slate-900/55 shadow-[0_16px_45px_rgba(2,6,23,0.16)] transition duration-200 ease-out hover:-translate-y-0.5 hover:border-blue-400/25 hover:bg-slate-800/70 focus-within:border-blue-400/25 ${menuOpen ? 'z-100' : ''} ${draggable.isDragging ? 'z-40 opacity-60' : ''}`}
		>
			<button
				type='button'
				onClick={() => void onLaunch(app)}
				aria-label={`Launch ${app.name}`}
				title={app.path}
				className='flex min-h-34 w-full flex-col items-center justify-center gap-3 px-4 py-4 text-center focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-blue-400'
			>
				<span className='grid size-13 place-items-center rounded-xl bg-slate-950/55 ring-1 ring-inset ring-white/6'>
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
							className='text-slate-500 transition-colors group-hover:text-blue-300'
							aria-hidden='true'
						/>
					)}
				</span>
				<span className='w-full truncate text-sm font-medium text-slate-200 group-hover:text-slate-50'>
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
				className='absolute left-2 top-2 grid size-8 cursor-grab place-items-center rounded-lg border border-white/8 bg-slate-950/85 text-slate-400 opacity-75 transition hover:text-blue-300 hover:opacity-100 focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-blue-400 active:cursor-grabbing'
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
				className={`absolute right-2 top-2 grid size-8 place-items-center rounded-lg border transition focus-visible:opacity-100 focus-visible:outline-2 focus-visible:outline-yellow-300 ${isFavorite ? 'border-yellow-300/25 bg-yellow-300/12 text-yellow-300 opacity-100' : 'border-white/8 bg-slate-950/85 text-slate-400 opacity-0 hover:text-yellow-200 group-hover:opacity-100 group-focus-within:opacity-100'}`}
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
