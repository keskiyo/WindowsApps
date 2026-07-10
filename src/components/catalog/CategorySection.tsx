import { useDroppable } from '@dnd-kit/core'
import { ChevronDown, ChevronRight, Pencil, Trash2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import type { AppCategory, AppInfo, CategoryDefinition } from '../../types'
import { AppCard } from '../apps/AppCard'
import { DeleteCategoryDialog } from '../dialogs/DeleteCategoryDialog'
import { CategoryNameEditor } from '../shared/CategoryNameEditor'

interface Props {
	category: AppCategory
	definition: CategoryDefinition
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	apps: AppInfo[]
	collapsed: boolean
	favoriteAppIds: string[]
	dragActivator: ReactNode
	onToggle(): void
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	onMoveApp(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onHide(id: string): void
	onRestore(id: string): void
	onRenameCategory(
		id: string,
		label: string,
	): { ok: true } | { ok: false; error: string }
	onDeleteCategory(id: string): { ok: true } | { ok: false; error: string }
}

export function CategorySection({
	category,
	definition,
	categories,
	categoryOrder,
	apps,
	collapsed,
	favoriteAppIds,
	dragActivator,
	onToggle,
	onToggleFavorite,
	onLaunch,
	onMoveApp,
	onInfo,
	onUninstall,
	onHide,
	onRestore,
	onRenameCategory,
	onDeleteCategory,
}: Props) {
	const label = definition.label
	const [editing, setEditing] = useState(false)
	const [deleting, setDeleting] = useState(false)
	const drop = useDroppable({
		id: `category-drop:${category}`,
		data: { type: 'category', category },
	})
	return (
		<section
			ref={drop.setNodeRef}
			aria-labelledby={`category-${category}`}
			data-category={category}
			className={`relative scroll-mt-40 rounded-2xl transition-colors duration-200 focus-within:z-90 lg:scroll-mt-24 ${drop.isOver ? 'bg-violet-100/55 ring-1 ring-violet-400/35' : ''}`}
		>
			<div className='mb-3 flex items-center gap-2'>
				<button
					type='button'
					aria-expanded={!collapsed}
					aria-label={`${collapsed ? 'Expand' : 'Collapse'} ${label}`}
					onClick={onToggle}
					disabled={apps.length === 0}
					className='grid size-8 shrink-0 place-items-center rounded-lg bg-slate-200/75 text-slate-500 transition-colors hover:text-violet-700 focus-visible:outline-2 focus-visible:outline-violet-500 disabled:pointer-events-none disabled:opacity-40'
				>
					{collapsed ? (
						<ChevronRight size={15} aria-hidden='true' />
					) : (
						<ChevronDown size={15} aria-hidden='true' />
					)}
				</button>
				{editing ? (
					<CategoryNameEditor
						initialValue={label}
						label={`Rename ${label} category`}
						onCancel={() => setEditing(false)}
						onSave={value => {
							const result = onRenameCategory(category, value)
							if (result.ok) setEditing(false)
							return result.ok ? null : result.error
						}}
					/>
				) : (
					dragActivator
				)}
				{!editing && (
					<button
						type='button'
						aria-label={`Rename ${label} category`}
						onClick={() => setEditing(true)}
						className='grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-violet-100/75 hover:text-violet-700 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<Pencil size={15} />
					</button>
				)}
				{!definition.builtIn && (
					<button
						type='button'
						aria-label={`Delete ${label} category`}
						onClick={() => setDeleting(true)}
						className='grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-red-100 hover:text-red-700 focus-visible:outline-2 focus-visible:outline-red-500'
					>
						<Trash2 size={15} />
					</button>
				)}
			</div>
			{!collapsed && (
				<div className='grid grid-cols-2 gap-3.5 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'>
					{apps.map(app => (
						<AppCard
							key={app.id}
							app={app}
							isFavorite={favoriteAppIds.includes(app.id)}
							categories={categories}
							categoryOrder={categoryOrder}
							onToggleFavorite={onToggleFavorite}
							onLaunch={onLaunch}
							onMove={onMoveApp}
							onInfo={onInfo}
							onUninstall={onUninstall}
							onHide={onHide}
							onRestore={onRestore}
						/>
					))}
				</div>
			)}
			{deleting && (
				<DeleteCategoryDialog
					name={label}
					onClose={() => setDeleting(false)}
					onConfirm={() => {
						onDeleteCategory(category)
						setDeleting(false)
					}}
				/>
			)}
		</section>
	)
}
