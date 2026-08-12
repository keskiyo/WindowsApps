import { formatBytes } from './format'
import type { InstallProgressProps } from './types'

export function InstallProgress({
	phase,
	progress,
	downloadedBytes,
	effectiveTotal,
}: InstallProgressProps) {
	const stage = phase[0].toUpperCase() + phase.slice(1)
	return (
		<div className="mt-5" aria-label="Update progress" aria-live="polite">
			<div className="mb-3 text-[11px] font-medium text-violet-200">
				{stage}
			</div>
			<div className="h-2 overflow-hidden rounded-full bg-slate-900/50">
				<div
					className="h-full rounded-full bg-violet-400 transition-[width]"
					style={{ width: `${progress ?? 0}%` }}
				/>
			</div>
			{phase === 'downloading' && effectiveTotal && (
				<div className="mt-2 flex justify-between text-xs text-slate-300">
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
