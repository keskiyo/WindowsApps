import { Search } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import { type AppInfo, rankAppsByQueryTop } from '../../../entities/app'
import { useBodyScrollLock } from '../../../shared/hooks/useBodyScrollLock'
import { useFocusTrap } from '../../../shared/hooks/useFocusTrap'

const MAX_RESULTS = 50

interface Props {
	apps: AppInfo[]
	label: string
	onSelect(app: AppInfo): void
	onClose(): void
}

/**
 * Picks one application out of the catalog for a scenario list.
 *
 * Deliberately a separate dialog from the quick-launch palette rather than a mode of it: this one
 * selects and that one launches, and a picker that could start an app by mistake is the wrong
 * thing to put behind a row the user is still editing. The keyboard contract is the same, so the
 * ranking, the scroll lock and the focus trap are the catalog's own.
 */
export function AppPickerDialog({ apps, label, onSelect, onClose }: Props) {
	useBodyScrollLock()
	const dialogRef = useRef<HTMLDivElement>(null)
	const inputRef = useRef<HTMLInputElement>(null)
	const listRef = useRef<HTMLUListElement>(null)
	const [query, setQuery] = useState('')
	const [selected, setSelected] = useState(0)
	useFocusTrap(dialogRef)

	const results = useMemo(
		() => rankAppsByQueryTop(apps, query, MAX_RESULTS),
		[apps, query],
	)

	// Capture the trigger before focus moves: reading `activeElement` during cleanup would find
	// this dialog's own input, which is detached by then, and a keyboard user would lose the row.
	useEffect(() => {
		const trigger = document.activeElement
		inputRef.current?.focus()
		return () => {
			if (trigger instanceof HTMLElement && trigger.isConnected)
				trigger.focus()
		}
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
				results.length ? (value - 1 + results.length) % results.length : 0,
			)
		} else if (event.key === 'Enter') {
			event.preventDefault()
			const app = results[selected]
			if (app) onSelect(app)
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
				aria-label={label}
				onKeyDown={onKeyDown}
				className='motion-panel flex h-[min(24rem,calc(100vh-7rem))] w-full max-w-xl flex-col overflow-hidden rounded-2xl border border-(--border-neutral) bg-(--surface-panel) shadow-(--shadow-palette)'
			>
				<div className='flex items-center gap-3 border-b border-(--border-neutral) px-4'>
					<Search size={18} aria-hidden='true' />
					<input
						ref={inputRef}
						value={query}
						onChange={event => setQuery(event.target.value)}
						placeholder='Search apps…'
						role='combobox'
						aria-expanded='true'
						aria-controls='scenario-picker-list'
						aria-label={label}
						className='h-13 w-full bg-transparent text-sm text-(--text-primary) outline-none placeholder:text-(--text-subtle)'
					/>
				</div>
				<ul
					ref={listRef}
					id='scenario-picker-list'
					role='listbox'
					aria-label='Applications'
					className='min-h-0 flex-1 overflow-y-auto overscroll-contain p-2'
				>
					{results.length === 0 ? (
						<li className='px-3 py-6 text-center text-sm text-(--text-muted)'>
							No apps match “{query}”
						</li>
					) : (
						results.map((app, index) => (
							<li key={app.id} role='option' aria-selected={index === selected}>
								<button
									type='button'
									onMouseMove={() => setSelected(index)}
									onClick={() => onSelect(app)}
									className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm ${index === selected ? 'palette-result-selected font-medium' : 'text-(--text-primary) hover:bg-(--surface-raised)'}`}
								>
									<span className='grid size-7 shrink-0 place-items-center rounded-md bg-(--surface-inset)'>
										{app.iconBase64 ? (
											<img
												src={app.iconBase64}
												alt=''
												className='size-5 object-contain'
											/>
										) : null}
									</span>
									<span className='min-w-0 flex-1 truncate'>{app.name}</span>
								</button>
							</li>
						))
					)}
				</ul>
			</div>
		</div>
	)
}
