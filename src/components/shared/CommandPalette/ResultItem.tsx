import { AppWindow, CornerDownLeft } from 'lucide-react'
import type { ResultItemProps } from './types'

export function ResultItem({
	app,
	selected,
	onHover,
	onActivate,
}: ResultItemProps) {
	return (
		<li
			id={`cp-option-${app.id}`}
			role='option'
			aria-selected={selected}
			onMouseMove={onHover}
			onClick={onActivate}
			className={`flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors ${selected ? 'bg-violet-500/24 font-medium text-slate-100' : 'text-slate-700 hover:bg-slate-500/10'}`}
		>
			<span className='grid size-7 shrink-0 place-items-center rounded-md bg-white/70 ring-1 ring-inset ring-slate-200'>
				{app.iconBase64 ? (
					<img
						src={app.iconBase64}
						alt=''
						className='size-5 object-contain'
					/>
				) : (
					<AppWindow
						size={15}
						className='text-slate-500'
						aria-hidden='true'
					/>
				)}
			</span>
			<span className='min-w-0 flex-1 truncate'>{app.name}</span>
			{selected && (
				<CornerDownLeft
					size={14}
					className='shrink-0 text-violet-200'
					aria-hidden='true'
				/>
			)}
		</li>
	)
}
