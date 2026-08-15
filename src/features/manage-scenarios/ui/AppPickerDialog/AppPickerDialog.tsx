import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { categoryLabel } from '../../../../entities/category'
import { useBodyScrollLock } from '../../../../shared/hooks/useBodyScrollLock'
import { useFocusTrap } from '../../../../shared/hooks/useFocusTrap'
import { useProgressiveList } from '../../../../shared/hooks/useProgressiveList'
import type { AppPickerDialogProps } from '../../types'
import { AppPickerHeader } from './AppPickerHeader'
import { AppPickerRow } from './AppPickerRow'
import { useAppPickerSelection } from './useAppPickerSelection'
import { usePickerResults } from './usePickerResults'

const REVEAL_MARGIN = 160

export function AppPickerDialog({
	apps,
	categories,
	list,
	scenarioName,
	noteOf,
	markOf,
	onConfirm,
	onClose,
}: AppPickerDialogProps) {
	useBodyScrollLock()
	const dialogRef = useRef<HTMLDivElement>(null)
	const inputRef = useRef<HTMLInputElement>(null)
	const [query, setQuery] = useState('')
	const selection = useAppPickerSelection()
	useFocusTrap(dialogRef)

	const label = `Add an app to the ${list} list of ${scenarioName}`
	const results = usePickerResults(apps, categories, query)
	const rows = useProgressiveList(results)
	const count = selection.selected.length

	useEffect(() => {
		const trigger = document.activeElement
		inputRef.current?.focus()
		return () => {
			if (trigger instanceof HTMLElement && trigger.isConnected)
				trigger.focus()
		}
	}, [])

	return createPortal(
		<div
			className="motion-overlay fixed inset-0 z-500 grid place-items-center bg-slate-700/40 p-4 backdrop-blur-[2px]"
			onMouseDown={event => {
				if (event.currentTarget === event.target) onClose()
			}}
		>
			<div
				ref={dialogRef}
				role="dialog"
				aria-modal="true"
				aria-label={label}
				onKeyDown={event => {
					if (event.key !== 'Escape') return
					event.preventDefault()
					onClose()
				}}
				className="motion-panel flex h-[min(34rem,calc(100vh-2rem))] w-[min(36rem,calc(100vw-2rem))] flex-col overflow-hidden rounded-2xl border border-(--border-neutral) bg-(--surface-panel) shadow-(--shadow-palette)"
			>
				<AppPickerHeader
					list={list}
					scenarioName={scenarioName}
					label={label}
					query={query}
					inputRef={inputRef}
					onQueryChange={setQuery}
				/>
				<ul
					id="scenario-picker-list"
					aria-label="Applications"
					onScroll={event => {
						if (!rows.hasMore) return
						const list = event.currentTarget
						if (
							list.scrollTop + list.clientHeight >=
							list.scrollHeight - REVEAL_MARGIN
						)
							rows.revealMore()
					}}
					className="min-h-0 flex-1 overflow-y-auto overscroll-contain p-2"
				>
					{results.length === 0 && (
						<li className="px-3 py-6 text-center text-sm text-(--text-muted)">
							No apps match “{query}”
						</li>
					)}
					{rows.visible.map(app => {
						const note = noteOf?.(app) ?? null
						return (
							<AppPickerRow
								key={app.id}
								app={app}
								caption={
									note ?? categoryLabel(categories, app.category)
								}
								checked={selection.isSelected(app)}
								disabled={note !== null}
								mark={markOf?.(app) ?? null}
								onToggle={() => selection.toggle(app)}
							/>
						)
					})}
					{rows.hasMore && (
						<li className="px-1 py-2">
							<button
								type="button"
								onClick={rows.revealMore}
								className="w-full rounded-lg border border-(--border-neutral) bg-(--surface-inset) px-3 py-2 text-xs font-medium text-(--text-primary) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
							>
								Show {rows.remaining} more
							</button>
						</li>
					)}
				</ul>
				<div className="flex items-center justify-between gap-3 border-t border-(--border-neutral) px-4 py-3">
					<p aria-live="polite" className="text-xs text-(--text-muted)">
						{count} selected of {results.length}
					</p>
					<div className="flex shrink-0 gap-2">
						<button
							type="button"
							onClick={onClose}
							className="inline-flex h-8 items-center rounded-lg border border-(--border-neutral) bg-(--surface-panel) px-3 text-xs font-medium text-(--text-primary) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
						>
							Cancel
						</button>
						<button
							type="button"
							aria-label="Add selected apps"
							disabled={count === 0}
							onClick={() => onConfirm(selection.selected)}
							className="inline-flex h-8 items-center rounded-lg border border-(--accent) bg-(--utility-accent) px-3 text-xs font-medium text-(--text-primary) hover:bg-(--utility-accent-hover) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:cursor-not-allowed disabled:opacity-60"
						>
							{count ? `Add ${count}` : 'Add'}
						</button>
					</div>
				</div>
			</div>
		</div>,
		document.body,
	)
}
