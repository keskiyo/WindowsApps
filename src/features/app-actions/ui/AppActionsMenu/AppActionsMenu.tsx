import {
	ArrowRight,
	EyeOff,
	Info,
	RotateCcw,
	Trash2,
	Wrench,
} from 'lucide-react'
import { useState, type CSSProperties } from 'react'
import { createPortal } from 'react-dom'
import { isCatalogArtifact } from '../../../../entities/app'
import { CategorySubmenu } from './CategorySubmenu'
import { MenuItem } from './MenuItem'
import type { AppActionsMenuProps } from './types'
import { useActionsMenu } from '../../model/useActionsMenu'

export function AppActionsMenu({
	app,
	categories,
	categoryOrder,
	onClose,
	onMove,
	onInfo,
	onUninstall,
	isHidden = false,
	isUserPromoted = false,
	onHide,
	onRestore,
	onDemote,
	anchorRef,
}: AppActionsMenuProps) {
	const [showCategories, setShowCategories] = useState(false)
	const artifact = isCatalogArtifact(app) && !app.userInstaller
	const {
		menuRef,
		categoryMenuRef,
		position,
		categoryPosition,
		onMenuKeyDown,
	} = useActionsMenu({
		anchorRef,
		onClose,
		showCategories,
	})
	return createPortal(
		<>
			<div
				ref={menuRef}
				onKeyDown={onMenuKeyDown}
				style={
					{
						left: position.left,
						top: position.top,
						'--spotlight-opacity': 0,
					} as CSSProperties
				}
				role="menu"
				aria-label={`${app.name} actions`}
				className="motion-panel fixed z-[600] flex max-h-[calc(100vh-1.5rem)] w-56 max-w-[calc(100vw-1.5rem)] flex-col gap-0.5 overflow-y-auto rounded-xl border border-slate-200/85 bg-slate-50 p-2 text-left text-slate-700 shadow-(--shadow-menu)"
			>
				{!isHidden && !artifact && (
					<>
						<MenuItem
							trailingIcon={ArrowRight}
							label="Move to category"
							onClick={() => setShowCategories(value => !value)}
						/>
						<div
							role="separator"
							className="mx-1 my-1 border-t border-slate-200/55"
						/>
					</>
				)}
				<MenuItem
					icon={Info}
					label="App info"
					onClick={() => {
						onClose()
						onInfo(app)
					}}
				/>
				<MenuItem
					icon={
						isHidden ? RotateCcw : isUserPromoted ? Wrench : EyeOff
					}
					label={
						isHidden
							? 'Restore to catalog'
							: isUserPromoted
								? 'Move back to Auxiliary tools'
								: 'Hide from catalog'
					}
					onClick={() => {
						if (isHidden) onRestore(app.id)
						else if (isUserPromoted) onDemote(app.id)
						else onHide(app.id)
						onClose()
					}}
				/>
				{!isHidden && (
					<div className="mx-1 my-1 border-t border-slate-200/55" />
				)}
				{!isHidden && app.canUninstall && (
					<MenuItem
						icon={Trash2}
						tone="danger"
						withSpotlight={false}
						label="Uninstall"
						onClick={() => {
							onUninstall(app)
							onClose()
						}}
					/>
				)}
				{!isHidden && !app.canUninstall && (
					<MenuItem
						icon={Trash2}
						disabled
						withSpotlight={false}
						label="Uninstall unavailable"
					/>
				)}
			</div>
			{!isHidden && !artifact && showCategories && (
				<CategorySubmenu
					categories={categories}
					categoryOrder={categoryOrder}
					activeCategory={app.category}
					menuRef={categoryMenuRef}
					position={categoryPosition}
					onKeyDown={onMenuKeyDown}
					label={`Move ${app.name} to category`}
					onSelect={category => {
						onMove(app.id, category)
						onClose()
					}}
				/>
			)}
		</>,
		document.querySelector<HTMLElement>('.app-shell') ?? document.body,
	)
}
