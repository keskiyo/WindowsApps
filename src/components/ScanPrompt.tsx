import { ScanSearch, X } from 'lucide-react'

interface Props {
	isScanning: boolean
	onScan(): Promise<void>
	onDismiss(): void
}

export function ScanPrompt({ isScanning, onScan, onDismiss }: Props) {
	return (
		<section className='grid min-h-[55vh] place-items-center px-4 text-center'>
			<div className='relative w-full max-w-md rounded-2xl border border-white/8 bg-slate-900/55 px-8 py-10'>
				<button
					type='button'
					aria-label='Dismiss scan prompt'
					onClick={onDismiss}
					className='absolute right-3 top-3 grid size-9 place-items-center rounded-lg text-slate-500 hover:bg-slate-800 hover:text-slate-200 focus-visible:outline-2 focus-visible:outline-blue-400'
				>
					<X size={17} aria-hidden='true' />
				</button>
				<ScanSearch
					className='mx-auto text-blue-300'
					size={36}
					aria-hidden='true'
				/>
				<h2 className='mt-5 text-lg font-semibold'>
					Find your applications
				</h2>
				<p className='mx-auto mt-2 max-w-xs text-sm leading-6 text-slate-400'>
					Scan Windows when you are ready. Nothing runs automatically
					at startup.
				</p>
				<button
					type='button'
					disabled={isScanning}
					onClick={() => void onScan()}
					className='mt-6 rounded-xl bg-blue-500 px-5 py-2.5 text-sm font-semibold text-slate-950 hover:bg-blue-400 focus-visible:outline-2 focus-visible:outline-blue-300 disabled:opacity-60'
				>
					{isScanning ? 'Scanning applications…' : 'Scan for apps'}
				</button>
			</div>
		</section>
	)
}
