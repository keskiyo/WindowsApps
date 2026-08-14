import { Trash2 } from 'lucide-react'
import { useState } from 'react'
import { CatalogAppCard } from '../CatalogAppCard/CatalogAppCard'
import {
	CategoryNameEditor,
	DeleteCategoryDialog,
} from '../../../../features/manage-category'
import { CollapsiblePanel } from '../../../../shared/ui/CollapsiblePanel'
import { CategoryHeader } from './CategoryHeader'
import type { CategorySectionProps } from './types'
import { useProgressiveCards } from './useProgressiveCards'

export function CategorySection({
	category,
	definition,
	categories,
	categoryOrder,
	apps,
	collapsed,
	favoriteIds,
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
	const cards = useProgressiveCards(apps)
	return (
		<section
			aria-labelledby={`category-${category}`}
			data-category={category}
			className="relative scroll-mt-40 rounded-2xl transition-colors duration-200 focus-within:z-90 lg:scroll-mt-24"
		>
			<div className="mb-3 flex items-center gap-2">
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
						onToggle={onToggle}
						onEdit={() => setEditing(true)}
					/>
				)}
				{!definition.builtIn && (
					<button
						type="button"
						aria-label={`Delete ${label} category`}
						onClick={() => setDeleting(true)}
						className="grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-red-100 hover:text-red-700 focus-visible:outline-2 focus-visible:outline-red-500"
					>
						<Trash2 size={15} />
					</button>
				)}
			</div>
			<CollapsiblePanel open={!collapsed}>
				<div>
					<div className="app-card-grid">
						{cards.visible.map(app => (
							<CatalogAppCard
								key={app.id}
								app={app}
								isFavorite={favoriteIds.has(app.id)}
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
					{cards.hasMore && (
						<div
							ref={cards.sentinelRef}
							aria-hidden="true"
							className="h-1"
						/>
					)}
				</div>
			</CollapsiblePanel>
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
