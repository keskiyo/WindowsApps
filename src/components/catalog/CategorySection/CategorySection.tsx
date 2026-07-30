import { useDroppable } from '@dnd-kit/core'
import { Trash2 } from 'lucide-react'
import { useState } from 'react'
import { AppCard } from '../../apps/AppCard/AppCard'
import { DeleteCategoryDialog } from '../../dialogs/DeleteCategoryDialog'
import { CategoryNameEditor } from '../../shared/CategoryNameEditor'
import { CategoryHeader } from './CategoryHeader'
import type { CategorySectionProps } from './types'

export function CategorySection({
	category,
	definition,
	categories,
	categoryOrder,
	apps,
	collapsed,
	favoriteAppIds,
	titlePointerDown,
	onToggle,
	onToggleFavorite,
	onLaunch,
	onMoveApp,
	onInfo,
	onUninstall,
	onHide,
	onRestore,
	onDemote,
	onRenameCategory,
	onDeleteCategory,
}: CategorySectionProps) {
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
					<CategoryHeader
						category={category}
						label={label}
						appCount={apps.length}
						collapsed={collapsed}
						titlePointerDown={titlePointerDown}
						onToggle={onToggle}
						onEdit={() => setEditing(true)}
					/>
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
							onDemote={onDemote}
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
