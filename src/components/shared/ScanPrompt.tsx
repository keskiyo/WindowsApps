import { ScanSearch, X } from 'lucide-react'

interface Props {
	isScanning: boolean
	onScan(): Promise<void>
	onDismiss(): void
}

export function ScanPrompt({ isScanning, onScan, onDismiss }: Props) {
	return (
		<section className='grid min-h-[55vh] place-items-center px-4 text-center'>
			<div className='relative w-full max-w-md rounded-2xl border border-white/85 bg-white/58 px-8 py-10 shadow-[8px_9px_22px_rgba(104,114,136,.13),-7px_-7px_18px_rgba(255,255,255,.78)] backdrop-blur-xl'>
				<button
					type='button'
					aria-label='Dismiss scan prompt'
					onClick={onDismiss}
					className='absolute right-3 top-3 grid size-9 place-items-center rounded-lg text-slate-500 hover:bg-slate-200/75 hover:text-slate-800 focus-visible:outline-2 focus-visible:outline-violet-500'
				>
					<X size={17} aria-hidden='true' />
				</button>
				<ScanSearch
					className='mx-auto text-violet-600'
					size={36}
					aria-hidden='true'
				/>
				<h2 className='mt-5 text-lg font-semibold'>
					Find your applications
				</h2>
				<p className='mx-auto mt-2 max-w-xs text-sm leading-6 text-slate-600'>
					Scan Windows when you are ready. Nothing runs automatically
					at startup.
				</p>
				<button
					type='button'
					disabled={isScanning}
					onClick={() => void onScan()}
					className='mt-6 rounded-xl bg-violet-600 px-5 py-2.5 text-sm font-semibold text-white shadow-[0_8px_18px_rgba(104,69,216,.22)] hover:bg-violet-500 focus-visible:outline-2 focus-visible:outline-violet-500 disabled:opacity-60'
				>
					{isScanning ? 'Scanning applications…' : 'Scan for apps'}
				</button>
			</div>
		</section>
	)
}
