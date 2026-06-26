import { FolderSearch, Trash2 } from 'lucide-react'
import type { ReactNode } from 'react'
import { useSpotlight } from '../../hooks/useSpotlight'
import { SpotlightLayer } from '../shared/SpotlightLayer'

export interface PathEditorProps {
	label: string
	buttonLabel: string
	browseLabel: string
	value: string
	paths: string[]
	icon: ReactNode
	disabled: boolean
	onChange(value: string): void
	onAdd(value: string): void
	onBrowse(): Promise<string | null>
	onRemove(value: string): void
}

export function PathEditor(props: PathEditorProps) {
	const spotlight = useSpotlight()
	async function browse() {
		if (props.disabled) return
		const picked = await props.onBrowse()
		if (picked) props.onAdd(picked)
	}
	return (
		<div>
			<label className='text-sm font-medium' htmlFor={props.label}>
				{props.label}
			</label>
			<div className='mt-2 flex gap-2'>
				<div
					className='relative min-w-0 flex-1 rounded-xl'
					onPointerMove={spotlight.onPointerMove}
					onPointerEnter={spotlight.onPointerEnter}
					onPointerLeave={spotlight.onPointerLeave}
				>
					<SpotlightLayer size={150} />
					<input
						id={props.label}
						aria-label={props.label}
						value={props.value}
						onChange={event => props.onChange(event.target.value)}
						onDoubleClick={() => void browse()}
						placeholder='D:\\Apps'
						title='Double-click to browse for a folder'
						className='h-10 w-full rounded-xl border border-slate-200 bg-white/75 px-3 text-sm text-slate-800 outline-none focus:border-violet-400/55 focus:ring-3 focus:ring-violet-500/10'
					/>
				</div>
				<button
					type='button'
					aria-label={props.browseLabel}
					disabled={props.disabled}
					onClick={() => void browse()}
					className='grid size-10 shrink-0 place-items-center rounded-xl border border-slate-200 bg-white/75 text-violet-700 hover:bg-violet-50 focus-visible:outline-2 focus-visible:outline-violet-500 disabled:opacity-40'
				>
					<FolderSearch size={16} aria-hidden='true' />
				</button>
			</div>
			<ul className='mt-2 space-y-1'>
				{props.paths.map(path => (
					<li
						key={path}
						className='flex items-center gap-2 rounded-lg bg-slate-100/85 px-2.5 py-2 text-xs text-slate-600'
					>
						<code className='min-w-0 flex-1 truncate'>{path}</code>
						<button
							type='button'
							aria-label={`Remove ${path}`}
							onClick={() => props.onRemove(path)}
							className='grid size-7 place-items-center rounded-md hover:bg-red-100 hover:text-red-700'
						>
							<Trash2 size={14} aria-hidden='true' />
						</button>
					</li>
				))}
			</ul>
		</div>
	)
}
