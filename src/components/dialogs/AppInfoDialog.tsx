import { AppWindow, X } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { useBodyScrollLock } from '../../hooks/useBodyScrollLock'
import { descriptionLabel, metadataRows } from '../../lib/appMetadata'
import type { AppInfo, CategoryDefinition } from '../../types'

export function AppInfoDialog({
	app,
	categories,
	onClose,
}: {
	app: AppInfo
	categories: CategoryDefinition[]
	onClose(): void
}) {
	useBodyScrollLock()
	const closeRef = useRef<HTMLButtonElement>(null)
	useEffect(() => {
		closeRef.current?.focus()
		function keydown(event: KeyboardEvent) {
			if (event.key === 'Escape') onClose()
		}
		document.addEventListener('keydown', keydown)
		return () => document.removeEventListener('keydown', keydown)
	}, [onClose])
	const rows = metadataRows(app, categories)
	return (
		<div
			className='fixed inset-0 z-400 grid place-items-center bg-slate-950/78 p-4'
			onMouseDown={event => {
				if (event.currentTarget === event.target) onClose()
			}}
		>
			<section
				role='dialog'
				aria-modal='true'
				aria-label={`${app.name} information`}
				className='max-h-[85vh] w-full max-w-xl overflow-y-auto rounded-2xl border border-white/10 bg-slate-900 p-5 shadow-2xl shadow-black/50'
			>
				<header className='flex items-start gap-4'>
					<span className='grid size-13 shrink-0 place-items-center rounded-xl bg-slate-950/70'>
						{app.iconBase64 ? (
							<img
								src={app.iconBase64}
								alt=''
								className='size-9.5 object-contain'
							/>
						) : (
							<AppWindow size={26} className='text-slate-500' />
						)}
					</span>
					<div className='min-w-0 flex-1'>
						<h2 className='truncate text-lg font-semibold'>
							{app.name}
						</h2>
						<p className='mt-1 text-sm leading-6 text-slate-400'>
							{descriptionLabel(app.description)}
						</p>
					</div>
					<button
						ref={closeRef}
						type='button'
						aria-label='Close app information'
						onClick={onClose}
						className='grid size-9 place-items-center rounded-lg text-slate-400 hover:bg-slate-800 hover:text-slate-100 focus-visible:outline-2 focus-visible:outline-blue-400'
					>
						<X size={17} />
					</button>
				</header>
				<dl className='mt-5 divide-y divide-white/7 rounded-xl border border-white/7 bg-slate-950/35 px-4'>
					{rows.map(([label, value]) => (
						<div
							key={label}
							className='grid gap-1 py-3 sm:grid-cols-[9rem_1fr]'
						>
							<dt className='text-xs font-medium uppercase tracking-wide text-slate-500'>
								{label}
							</dt>
							<dd className='select-text break-all text-sm text-slate-200'>
								{value}
							</dd>
						</div>
					))}
				</dl>
			</section>
		</div>
	)
}
