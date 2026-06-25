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
			className='fixed inset-0 z-400 grid place-items-center bg-slate-700/38 p-4 backdrop-blur-[2px]'
			onMouseDown={event => {
				if (event.currentTarget === event.target) onClose()
			}}
		>
			<section
				role='dialog'
				aria-modal='true'
				aria-label={`${app.name} information`}
				className='max-h-[85vh] w-full max-w-xl overflow-y-auto rounded-2xl border border-white/90 bg-slate-50 p-5 text-slate-800 shadow-[0_24px_70px_rgba(48,56,76,.28)]'
			>
				<header className='flex items-start gap-4'>
					<span className='grid size-13 shrink-0 place-items-center rounded-xl bg-slate-200/70 shadow-inner'>
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
						<p className='mt-1 text-sm leading-6 text-slate-600'>
							{descriptionLabel(app.description)}
						</p>
					</div>
					<button
						ref={closeRef}
						type='button'
						aria-label='Close app information'
						onClick={onClose}
						className='grid size-9 place-items-center rounded-lg text-slate-500 hover:bg-violet-100 hover:text-slate-900 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<X size={17} />
					</button>
				</header>
				<dl className='mt-5 divide-y divide-slate-200 rounded-xl border border-slate-200/90 bg-white/65 px-4'>
					{rows.map(([label, value]) => (
						<div
							key={label}
							className='grid gap-1 py-3 sm:grid-cols-[9rem_1fr]'
						>
							<dt className='text-xs font-medium uppercase tracking-wide text-slate-500'>
								{label}
							</dt>
							<dd className='select-text break-all text-sm text-slate-700'>
								{value}
							</dd>
						</div>
					))}
				</dl>
			</section>
		</div>
	)
}
