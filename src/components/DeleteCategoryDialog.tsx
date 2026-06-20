import { Trash2, X } from 'lucide-react'
import { useBodyScrollLock } from '../hooks/useBodyScrollLock'

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
	return (
		<div className='fixed inset-0 z-400 grid place-items-center bg-slate-950/80 p-4'>
			<section
				role='alertdialog'
				aria-modal='true'
				aria-label={`Delete ${name} category`}
				className='w-full max-w-md rounded-2xl border border-red-400/20 bg-slate-900 p-5 shadow-2xl'
			>
				<div className='flex items-start gap-3'>
					<Trash2 className='mt-0.5 text-red-300' size={20} />
					<div className='flex-1'>
						<h2 className='font-semibold'>Delete {name}?</h2>
						<p className='mt-2 text-sm leading-6 text-slate-400'>
							Applications in this category will move to Other.
						</p>
					</div>
					<button
						type='button'
						aria-label='Close category deletion'
						onClick={onClose}
					>
						<X size={17} />
					</button>
				</div>
				<div className='mt-5 flex justify-end gap-3'>
					<button
						type='button'
						onClick={onClose}
						className='rounded-xl border border-white/10 px-4 py-2 text-sm'
					>
						Cancel
					</button>
					<button
						type='button'
						onClick={onConfirm}
						className='rounded-xl bg-red-500 px-4 py-2 text-sm font-semibold text-slate-950'
					>
						Delete category
					</button>
				</div>
			</section>
		</div>
	)
}
