import {
	ChevronRight,
	EyeOff,
	Info,
	RotateCcw,
	Trash2,
	Wrench,
} from 'lucide-react'
import { useState, type CSSProperties } from 'react'
import { createPortal } from 'react-dom'
import { CategorySubmenu } from './CategorySubmenu'
import { MenuItem } from './MenuItem'
import type { AppActionsMenuProps } from './types'
import { useActionsMenu } from './useActionsMenu'

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
	const { menuRef, position, onMenuKeyDown } = useActionsMenu({
		anchorRef,
		onClose,
		showCategories,
	})
	return createPortal(
		<div
			ref={menuRef}
			onKeyDown={onMenuKeyDown}
			style={
				{
					left: position.left,
					top: position.top,
					// Reset the spotlight vars so menu items don't inherit the parent card's
					// glow (each item drives its own on hover).
					'--spotlight-opacity': 0,
				} as CSSProperties
			}
			role='menu'
			aria-label={`${app.name} actions`}
			className='motion-panel fixed z-[600] flex max-h-[calc(100vh-1.5rem)] w-56 max-w-[calc(100vw-1.5rem)] flex-col gap-0.5 overflow-y-auto rounded-xl border border-slate-200/85 bg-slate-50 p-2 text-left text-slate-700 shadow-[var(--shadow-menu)]'
		>
			{!isHidden && (
				<MenuItem
					icon={ChevronRight}
					iconClassName={`text-slate-400 transition-transform ${showCategories ? 'rotate-90' : ''}`}
					label='Move to category'
					onClick={() => setShowCategories(value => !value)}
				/>
			)}
			{!isHidden && showCategories && (
				<CategorySubmenu
					categories={categories}
					categoryOrder={categoryOrder}
					activeCategory={app.category}
					onSelect={category => {
						onMove(app.id, category)
						onClose()
					}}
				/>
			)}
			<MenuItem
				icon={Info}
				iconClassName='text-slate-400'
				label='App info'
				onClick={() => {
					onInfo(app)
					onClose()
				}}
			/>
			<MenuItem
				icon={isHidden ? RotateCcw : isUserPromoted ? Wrench : EyeOff}
				iconClassName='text-slate-400'
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
				<div className='mx-1 my-1 border-t border-slate-200/55' />
			)}
			{!isHidden && app.canUninstall && (
				<MenuItem
					icon={Trash2}
					tone='danger'
					withSpotlight={false}
					label='Uninstall'
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
					label='Uninstall unavailable'
				/>
			)}
		</div>,
		document.querySelector<HTMLElement>('.app-shell') ?? document.body,
	)
}
