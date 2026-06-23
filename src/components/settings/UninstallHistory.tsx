import { ShieldCheck, Trash2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import type { SystemClient, UninstallHistoryEntry } from '../../types'

interface Props {
	client: SystemClient
}

const METHOD_LABELS = {
	registered_command: 'Registered command',
	msi: 'MSI',
	msix: 'MSIX',
} as const

const RESULT_LABELS = {
	succeeded: 'Succeeded',
	failed: 'Failed',
} as const

const dateFormatter = new Intl.DateTimeFormat(undefined, {
	dateStyle: 'medium',
	timeStyle: 'short',
})

export function UninstallHistory({ client }: Props) {
	const [entries, setEntries] = useState<UninstallHistoryEntry[]>([])
	const [loading, setLoading] = useState(true)
	const [error, setError] = useState<string | null>(null)
	const [confirmClear, setConfirmClear] = useState(false)
	const [clearing, setClearing] = useState(false)

	useEffect(() => {
		let active = true
		setLoading(true)
		client
			.getUninstallHistory()
			.then(history => {
				if (active) setEntries(history)
			})
			.catch(reason => {
				if (active) setError(String(reason))
			})
			.finally(() => {
				if (active) setLoading(false)
			})
		return () => {
			active = false
		}
	}, [client])

	async function clearHistory() {
		setClearing(true)
		setError(null)
		try {
			await client.clearUninstallHistory()
			setEntries([])
			setConfirmClear(false)
		} catch (reason) {
			setError(String(reason))
		} finally {
			setClearing(false)
		}
	}

	return (
		<div className='mt-5 rounded-2xl border border-white/8 bg-slate-900/55 p-5'>
			<div className='flex items-start gap-4'>
				<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-slate-950/60 text-blue-300'>
					<ShieldCheck size={19} aria-hidden='true' />
				</span>
				<div className='min-w-0 flex-1'>
					<h2 className='font-medium'>Uninstall history</h2>
					<p className='mt-1 text-sm leading-6 text-slate-500'>
						Last 100 uninstall attempts. Commands, paths, arguments,
						errors and usernames are not stored.
					</p>
				</div>
				<button
					type='button'
					disabled={!entries.length || clearing}
					onClick={() => setConfirmClear(true)}
					className='rounded-xl border border-red-400/20 px-3 py-2 text-sm text-red-200 hover:bg-red-500/10 disabled:opacity-40'
				>
					Clear
				</button>
			</div>

			{loading ? (
				<p className='mt-4 text-sm text-slate-400'>Loading history…</p>
			) : error ? (
				<p role='alert' className='mt-4 text-sm text-red-300'>
					{error}
				</p>
			) : entries.length ? (
				<ul className='mt-4 divide-y divide-white/7 overflow-hidden rounded-xl border border-white/8 bg-slate-950/35'>
					{entries.map(entry => (
						<li key={entry.id} className='p-4'>
							<div className='flex flex-wrap items-center justify-between gap-2'>
								<p className='font-medium'>{entry.appName}</p>
								<span
									className={`rounded-full px-2.5 py-1 text-xs ${entry.result === 'succeeded' ? 'bg-emerald-500/10 text-emerald-200' : 'bg-red-500/10 text-red-200'}`}
								>
									{RESULT_LABELS[entry.result]}
								</span>
							</div>
							<p className='mt-2 text-sm text-slate-500'>
								{dateFormatter.format(new Date(entry.timestamp * 1000))}
							</p>
							<p className='mt-1 text-sm text-slate-400'>
								{entry.publisher ?? 'Unknown publisher'} ·{' '}
								{METHOD_LABELS[entry.mechanism]}
							</p>
						</li>
					))}
				</ul>
			) : (
				<p className='mt-4 text-sm text-slate-400'>
					No uninstall history yet.
				</p>
			)}

			{confirmClear && (
				<div
					role='dialog'
					aria-label='Confirm clear uninstall history'
					className='mt-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-red-400/20 bg-red-500/8 p-4'
				>
					<p className='text-sm text-red-100'>
						Clear the local uninstall history?
					</p>
					<div className='flex gap-2'>
						<button
							type='button'
							disabled={clearing}
							onClick={() => setConfirmClear(false)}
							className='rounded-lg px-3 py-2 text-sm text-slate-300 hover:bg-white/5'
						>
							Cancel
						</button>
						<button
							type='button'
							disabled={clearing}
							onClick={() => void clearHistory()}
							className='inline-flex items-center gap-2 rounded-lg bg-red-500 px-3 py-2 text-sm font-medium text-white hover:bg-red-400 disabled:opacity-50'
						>
							<Trash2 size={14} aria-hidden='true' />
							{clearing ? 'Clearing…' : 'Confirm clear'}
						</button>
					</div>
				</div>
			)}
		</div>
	)
}
