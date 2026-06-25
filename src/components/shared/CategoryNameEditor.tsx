import { Check, X } from 'lucide-react'
import { useState } from 'react'

interface Props {
	initialValue?: string
	label: string
	onSave(value: string): string | null
	onCancel(): void
}

export function CategoryNameEditor({
	initialValue = '',
	label,
	onSave,
	onCancel,
}: Props) {
	const [value, setValue] = useState(initialValue)
	const [error, setError] = useState<string | null>(null)
	function save() {
		const nextError = onSave(value)
		setError(nextError)
	}
	return (
		<div className='min-w-0 flex-1'>
			<div className='flex items-center gap-2'>
				<input
					autoFocus
					aria-label={label}
					value={value}
					onChange={event => setValue(event.target.value)}
					onKeyDown={event => {
						if (event.key === 'Enter') save()
						if (event.key === 'Escape') onCancel()
					}}
					className='h-9 min-w-0 flex-1 rounded-lg border border-violet-400/45 bg-white/80 px-3 text-sm text-slate-800 outline-none focus:ring-3 focus:ring-violet-500/10'
				/>
				<button
					type='button'
					aria-label='Save category name'
					onClick={save}
					className='grid size-9 place-items-center rounded-lg text-violet-700 hover:bg-violet-100'
				>
					<Check size={16} />
				</button>
				<button
					type='button'
					aria-label='Cancel category editing'
					onClick={onCancel}
					className='-mr-2 grid size-9 place-items-center rounded-lg text-slate-500 hover:bg-violet-100/75'
				>
					<X size={16} />
				</button>
			</div>
			{error && (
				<p className='mt-1 text-xs text-red-700' role='alert'>
					{error}
				</p>
			)}
		</div>
	)
}
