import { Check, X } from 'lucide-react'
import { useState } from 'react'

interface Props {
	initialValue?: string
	label: string
	onSave(value: string): string | null
	onCancel(): void
}

/**
 * Inline name field for a scenario. A near-twin of the category editor by design: the two carry
 * different validation messages and live in different features, and features do not reach across
 * to one another — two small implementations beat one shared field driven by a flag.
 */
export function ScenarioNameEditor({
	initialValue = '',
	label,
	onSave,
	onCancel,
}: Props) {
	const [value, setValue] = useState(initialValue)
	const [error, setError] = useState<string | null>(null)

	function save() {
		setError(onSave(value))
	}

	return (
		<div className='w-full min-w-0'>
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
					className='h-9 min-w-0 flex-1 rounded-lg border border-(--accent) bg-(--surface-inset) px-3 text-sm text-(--text-primary) outline-none'
				/>
				<button
					type='button'
					aria-label='Save scenario name'
					onClick={save}
					className='grid size-9 place-items-center rounded-lg hover:bg-(--surface-raised)'
				>
					<Check size={16} />
				</button>
				<button
					type='button'
					aria-label='Cancel scenario renaming'
					onClick={onCancel}
					className='grid size-9 place-items-center rounded-lg hover:bg-(--surface-raised)'
				>
					<X size={16} />
				</button>
			</div>
			{error && (
				<p className='mt-1 text-xs text-(--category-red)' role='alert'>
					{error}
				</p>
			)}
		</div>
	)
}
