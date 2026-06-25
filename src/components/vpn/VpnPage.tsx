import { ShieldCheck } from 'lucide-react'
import { useEffect, useState } from 'react'
import type { VpnClient, VpnInfo } from '../../types'

export function VpnPage({ client }: { client: VpnClient }) {
	const [items, setItems] = useState<VpnInfo[]>([])
	const [busy, setBusy] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)
	useEffect(() => {
		let active = true
		client
			.list()
			.then(value => active && setItems(value))
			.catch(reason => active && setError(String(reason)))
		return () => {
			active = false
		}
	}, [client])
	async function run(id: string, action: Promise<VpnInfo>) {
		setBusy(id)
		setError(null)
		try {
			const next = await action
			setItems(list => list.map(item => (item.id === id ? next : item)))
		} catch (reason) {
			setError(String(reason))
		} finally {
			setBusy(null)
		}
	}
	return (
		<section aria-labelledby='vpn-title' className='mx-auto max-w-3xl'>
			<h1 id='vpn-title' className='mb-6 text-2xl font-semibold'>
				VPN
			</h1>
			<div className='space-y-3'>
				{items.map(item => (
					<div
						key={item.id}
						className='flex items-center gap-4 rounded-2xl border border-white/8 bg-slate-900/55 p-5'
					>
						<span className='grid size-10 place-items-center rounded-xl bg-slate-950/60 text-violet-300'>
							<ShieldCheck size={19} aria-hidden='true' />
						</span>
						<div className='min-w-0 flex-1'>
							<h2 className='font-medium'>{item.name}</h2>
							<p className='mt-1 text-sm text-slate-500'>
								{!item.installed
									? 'Setup required'
									: item.connected
										? 'Connected'
										: 'Off'}
							</p>
						</div>
						{item.installed ? (
							<button
								type='button'
								role='switch'
								aria-label={`Toggle ${item.name}`}
								aria-checked={item.connected}
								disabled={busy === item.id}
								onClick={() => void run(item.id, client.set(item.id, !item.connected))}
								className={`relative h-7 w-12 shrink-0 rounded-full transition disabled:opacity-50 ${item.connected ? 'bg-violet-500' : 'bg-slate-700'}`}
							>
								<span
									className={`absolute left-1 top-1 size-5 rounded-full bg-white shadow transition-transform ${item.connected ? 'translate-x-5' : ''}`}
								/>
							</button>
						) : (
							<button
								type='button'
								aria-label={`Set up ${item.name}`}
								disabled={busy === item.id}
								onClick={() => void run(item.id, client.setup(item.id))}
								className='rounded-xl bg-violet-600 px-3 py-2 text-sm font-medium text-white hover:bg-violet-500 disabled:opacity-50'
							>
								Set up
							</button>
						)}
					</div>
				))}
			</div>
			{error && (
				<p role='alert' className='mt-4 text-sm text-red-300'>
					{error}
				</p>
			)}
		</section>
	)
}
