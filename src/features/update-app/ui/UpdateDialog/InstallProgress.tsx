import { UPDATE_STEPS } from './data'
import { formatBytes } from './format'
import type { InstallProgressProps } from './types'

export function InstallProgress({
	phase,
	progress,
	downloadedBytes,
	effectiveTotal,
}: InstallProgressProps) {
	const activeStep = UPDATE_STEPS.indexOf(
		phase as (typeof UPDATE_STEPS)[number],
	)
	return (
		<div className='mt-5' aria-label='Update progress' aria-live='polite'>
			<div className='mb-3 flex items-center justify-between gap-2 text-[11px] font-medium'>
				{UPDATE_STEPS.map((step, index) => (
					<span
						key={step}
						className={
							index <= activeStep
								? 'text-violet-200'
								: 'text-slate-500'
						}
					>
						{step[0].toUpperCase() + step.slice(1)}
					</span>
				))}
			</div>
			<div className='h-2 overflow-hidden rounded-full bg-slate-900/50'>
				<div
					className='h-full rounded-full bg-violet-400 transition-[width]'
					style={{ width: `${progress ?? 0}%` }}
				/>
			</div>
			{phase === 'downloading' && effectiveTotal && (
				<div className='mt-2 flex justify-between text-xs text-slate-300'>
					<span
						aria-label={`${formatBytes(downloadedBytes)} of ${formatBytes(effectiveTotal)}`}
					>
						{formatBytes(downloadedBytes)} of{' '}
						{formatBytes(effectiveTotal)}
					</span>
					<span>{progress ?? 0}%</span>
				</div>
			)}
		</div>
	)
}
