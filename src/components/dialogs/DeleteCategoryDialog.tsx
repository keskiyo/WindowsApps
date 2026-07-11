import { Trash2, X } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { useBodyScrollLock } from '../../hooks/useBodyScrollLock'
import { useFocusTrap } from '../../hooks/useFocusTrap'

export function DeleteCategoryDialog({
	name,
	onConfirm,
	onClose,
}: {
	name: string
	onConfirm(): void
	onClose(): void
}) {
	useBodyScrollLock()
	const dialogRef = useRef<HTMLElement>(null)
	const closeRef = useRef<HTMLButtonElement>(null)
	useFocusTrap(dialogRef)
	useEffect(() => {
		closeRef.current?.focus()
		function keydown(event: KeyboardEvent) {
			if (event.key === 'Escape') onClose()
		}
		document.addEventListener('keydown', keydown)
		return () => document.removeEventListener('keydown', keydown)
	}, [onClose])
	return (
		<div className='motion-overlay fixed inset-0 z-400 grid place-items-center bg-slate-700/38 p-4 backdrop-blur-[2px]'>
			<section
				ref={dialogRef}
				role='alertdialog'
				aria-modal='true'
				aria-label={`Delete ${name} category`}
				className='motion-panel w-full max-w-md rounded-2xl border border-red-300/55 bg-slate-50 p-5 text-slate-800 shadow-[0_24px_70px_rgba(48,56,76,.28)]'
			>
				<div className='flex items-start gap-3'>
					<Trash2 className='mt-0.5 text-red-600' size={20} />
					<div className='flex-1'>
						<h2 className='font-semibold'>Delete {name}?</h2>
						<p className='mt-2 text-sm leading-6 text-slate-600'>
							Applications in this category will move to Other.
						</p>
					</div>
					<button
						ref={closeRef}
						type='button'
						aria-label='Close category deletion'
						onClick={onClose}
						className='grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-violet-100 hover:text-slate-900 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<X size={17} />
					</button>
				</div>
				<div className='mt-5 flex justify-end gap-3'>
					<button
						type='button'
						onClick={onClose}
						className='rounded-xl border border-slate-300 px-4 py-2 text-sm text-slate-700 hover:bg-violet-100/70 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						Cancel
					</button>
					<button
						type='button'
						onClick={onConfirm}
						className='rounded-xl bg-red-500 px-4 py-2 text-sm font-semibold text-white'
					>
						Delete category
					</button>
				</div>
			</section>
		</div>
	)
}
