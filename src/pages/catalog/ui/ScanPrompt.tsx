import { Check, ScanSearch, X } from 'lucide-react'
import type { ScanPromptProps } from '../types'

const SCAN_SOURCES = [
	'Start Menu shortcuts and installed programs',
	'Microsoft Store apps and Steam games',
	'Portable executables in folders you choose',
]

export function ScanPrompt({
	isScanning,
	onScan,
	onDismiss,
	onConfigureFolders,
}: ScanPromptProps) {
	return (
		<section className="grid min-h-[55vh] place-items-center px-4 text-center">
			<div className="settings-surface relative w-full max-w-md rounded-2xl border border-white/85 bg-white/58 px-8 py-10 backdrop-blur-xl">
				<button
					type="button"
					aria-label="Dismiss scan prompt"
					onClick={onDismiss}
					className="absolute top-3 right-3 grid size-9 place-items-center rounded-lg text-slate-500 hover:bg-violet-100/75 hover:text-slate-800 focus-visible:outline-2 focus-visible:outline-violet-500"
				>
					<X size={17} aria-hidden="true" />
				</button>
				<ScanSearch
					className="mx-auto text-violet-600"
					size={36}
					aria-hidden="true"
				/>
				<h2 className="mt-5 text-lg font-semibold">
					Find your applications
				</h2>
				<p className="mx-auto mt-2 max-w-xs text-sm leading-6 text-slate-600">
					Scan Windows to build your catalog. Nothing runs
					automatically at startup.
				</p>
				<ul className="mx-auto mt-5 max-w-xs space-y-1.5 text-left text-sm text-slate-600">
					{SCAN_SOURCES.map(source => (
						<li key={source} className="flex items-start gap-2">
							<Check
								size={15}
								aria-hidden="true"
								className="mt-1 shrink-0 text-violet-600"
							/>
							{source}
						</li>
					))}
				</ul>
				<p className="mx-auto mt-4 max-w-xs text-xs leading-5 text-slate-500">
					The first scan usually takes under a minute. Everything
					stays on this device.
				</p>
				<button
					type="button"
					disabled={isScanning}
					onClick={() => void onScan()}
					className="utility-accent-button mt-6 rounded-xl px-5 py-2.5 text-sm font-semibold text-white focus-visible:outline-2 focus-visible:outline-violet-500 disabled:opacity-60"
				>
					{isScanning ? 'Scanning applications…' : 'Scan for apps'}
				</button>
				{onConfigureFolders && (
					<button
						type="button"
						onClick={onConfigureFolders}
						className="mt-3 block w-full rounded-lg px-3 py-1.5 text-sm text-slate-600 underline decoration-slate-300 underline-offset-4 hover:text-slate-900 focus-visible:outline-2 focus-visible:outline-violet-500"
					>
						Choose folders first
					</button>
				)}
			</div>
		</section>
	)
}
