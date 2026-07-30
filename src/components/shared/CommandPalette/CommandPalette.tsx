import { Search } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useBodyScrollLock } from '../../../hooks/useBodyScrollLock'
import { useFocusTrap } from '../../../hooks/useFocusTrap'
import { rankAppsByQuery } from '../../../store/appStore'
import { ResultItem } from './ResultItem'
import type { CommandPaletteProps } from './types'

const MAX_RESULTS = 50

/**
 * Ctrl+K quick-launch overlay (command palette pattern). Keyboard-first: type to filter,
 * arrows to move, Enter to launch, Escape to close. Reuses the catalog's own
 * `filterAppsByQuery` so ranking matches the main grid.
 */
export function CommandPalette({
	apps,
	onLaunch,
	onClose,
}: CommandPaletteProps) {
	useBodyScrollLock()
	const dialogRef = useRef<HTMLDivElement>(null)
	const inputRef = useRef<HTMLInputElement>(null)
	const listRef = useRef<HTMLUListElement>(null)
	const [query, setQuery] = useState('')
	const [selected, setSelected] = useState(0)
	useFocusTrap(dialogRef)

	const results = useMemo(
		() => rankAppsByQuery(apps, query).slice(0, MAX_RESULTS),
		[apps, query],
	)

	useEffect(() => {
		inputRef.current?.focus()
		const previous = document.activeElement as HTMLElement | null
		return () => previous?.focus?.()
	}, [])

	useEffect(() => {
		setSelected(value => (value >= results.length ? 0 : value))
	}, [results.length])

	useEffect(() => {
		const node = listRef.current?.children[selected] as
			| HTMLElement
			| undefined
		node?.scrollIntoView({ block: 'nearest' })
	}, [selected])

	function launch(app: (typeof results)[number]) {
		void onLaunch(app)
		onClose()
	}

	function onKeyDown(event: React.KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault()
			onClose()
		} else if (event.key === 'ArrowDown') {
			event.preventDefault()
			setSelected(value =>
				results.length ? (value + 1) % results.length : 0,
			)
		} else if (event.key === 'ArrowUp') {
			event.preventDefault()
			setSelected(value =>
				results.length
					? (value - 1 + results.length) % results.length
					: 0,
			)
		} else if (event.key === 'Enter') {
			event.preventDefault()
			const app = results[selected]
			if (app) launch(app)
		}
	}

	return (
		<div
			className='motion-overlay fixed inset-0 z-500 grid place-items-start justify-center bg-slate-700/40 px-4 pt-[14vh] backdrop-blur-[2px]'
			onMouseDown={event => {
				if (event.currentTarget === event.target) onClose()
			}}
		>
			<div
				ref={dialogRef}
				role='dialog'
				aria-modal='true'
				aria-label='Quick launch'
				onKeyDown={onKeyDown}
				className='motion-panel flex h-[min(22rem,calc(100vh-7rem))] w-full max-w-xl flex-col overflow-hidden rounded-2xl border border-white/90 bg-slate-50 shadow-(--shadow-palette)'
			>
				<div className='flex items-center gap-3 border-b border-slate-200 px-4'>
					<Search
						size={18}
						className='text-slate-500'
						aria-hidden='true'
					/>
					<input
						ref={inputRef}
						value={query}
						onChange={event => setQuery(event.target.value)}
						placeholder='Search apps to launch…'
						role='combobox'
						aria-expanded='true'
						aria-controls='command-palette-list'
						aria-activedescendant={
							results[selected]
								? `cp-option-${results[selected].id}`
								: undefined
						}
						aria-label='Quick launch search'
						className='h-13 w-full bg-transparent text-sm text-slate-800 outline-none placeholder:text-slate-500'
					/>
					<kbd className='hidden rounded border border-slate-300 bg-white px-1.5 py-0.5 text-[0.65rem] font-medium text-slate-500 sm:block'>
						Esc
					</kbd>
				</div>
				<ul
					ref={listRef}
					id='command-palette-list'
					role='listbox'
					aria-label='Applications'
					className='min-h-0 flex-1 overflow-y-auto overscroll-contain p-2'
				>
					{results.length === 0 ? (
						<li className='px-3 py-6 text-center text-sm text-slate-500'>
							No apps match “{query}”
						</li>
					) : (
						results.map((app, index) => (
							<ResultItem
								key={app.id}
								app={app}
								selected={index === selected}
								onHover={() => setSelected(index)}
								onActivate={() => launch(app)}
							/>
						))
					)}
				</ul>
			</div>
		</div>
	)
}
