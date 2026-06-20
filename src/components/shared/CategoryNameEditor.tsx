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
					className='h-9 min-w-0 flex-1 rounded-lg border border-blue-400/40 bg-slate-950 px-3 text-sm outline-none focus:ring-3 focus:ring-blue-500/10'
				/>
				<button
					type='button'
					aria-label='Save category name'
					onClick={save}
					className='grid size-9 place-items-center rounded-lg text-blue-300 hover:bg-slate-800'
				>
					<Check size={16} />
				</button>
				<button
					type='button'
					aria-label='Cancel category editing'
					onClick={onCancel}
					className='grid size-9 place-items-center rounded-lg text-slate-400 hover:bg-slate-800'
				>
					<X size={16} />
				</button>
			</div>
			{error && (
				<p className='mt-1 text-xs text-red-300' role='alert'>
					{error}
				</p>
			)}
		</div>
	)
}
