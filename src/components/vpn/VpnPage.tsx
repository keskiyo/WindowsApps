import { CircleAlert, LoaderCircle, ShieldCheck } from 'lucide-react'
import { useEffect, useState } from 'react'
import type { VpnClient, VpnInfo } from '../../types'

export function VpnPage({ client }: { client: VpnClient }) {
	const [items, setItems] = useState<VpnInfo[]>([])
	const [busy, setBusy] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)
	const [loading, setLoading] = useState(true)
	useEffect(() => {
		let active = true
		client
			.list()
			.then(value => active && setItems(value))
			.catch(reason => active && setError(String(reason)))
			.finally(() => active && setLoading(false))
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
			<div className='mb-7'>
				<h1 id='vpn-title' className='text-2xl font-semibold text-slate-800'>
					VPN
				</h1>
				<p className='mt-1.5 text-sm leading-6 text-slate-600'>
					Manage supported VPN applications without leaving Windows Apps.
				</p>
			</div>
			<div className='space-y-3'>
				{loading && <VpnSkeleton />}
				{items.map(item => (
					<div
						key={item.id}
						data-state={
							!item.installed
								? 'unavailable'
								: item.connected
									? 'connected'
									: 'disconnected'
						}
						className={`flex min-h-24 items-center gap-4 rounded-2xl border bg-white/58 p-5 shadow-[7px_8px_20px_rgba(104,114,136,.11),-6px_-6px_16px_rgba(255,255,255,.72)] transition-[border-color,background-color,box-shadow] duration-200 ${
							item.connected
								? 'border-violet-300/65 bg-violet-50/58 shadow-[7px_8px_20px_rgba(104,114,136,.10),-6px_-6px_16px_rgba(255,255,255,.72),0_0_0_1px_rgba(124,58,237,.05)]'
								: 'border-white/85'
						}`}
					>
						<span
							className={`grid size-11 shrink-0 place-items-center rounded-xl shadow-[inset_1px_1px_3px_rgba(111,124,146,.13),inset_-2px_-2px_5px_rgba(255,255,255,.78)] ${
								item.connected
									? 'bg-violet-100 text-violet-700 ring-1 ring-inset ring-violet-300/60'
									: 'bg-slate-200/70 text-slate-500 ring-1 ring-inset ring-white/80'
							}`}
						>
							<ShieldCheck size={19} aria-hidden='true' />
						</span>
						<div className='min-w-0 flex-1'>
							<h2 className='font-semibold text-slate-800'>{item.name}</h2>
							<p
								role='status'
								aria-live='polite'
								className={`mt-1 flex items-center gap-2 text-sm ${
									item.connected ? 'text-violet-700' : 'text-slate-500'
								}`}
							>
								<span
									className={`size-1.5 rounded-full ${
										item.connected
											? 'bg-violet-500 shadow-[0_0_0_3px_rgba(139,92,246,.12)]'
											: 'bg-slate-400'
									}`}
									aria-hidden='true'
								/>
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
								className={`relative h-7 w-12 shrink-0 rounded-full shadow-inner transition-colors duration-200 focus-visible:outline-2 focus-visible:outline-offset-3 focus-visible:outline-violet-500 disabled:cursor-wait disabled:opacity-55 ${item.connected ? 'bg-violet-600' : 'bg-slate-300'}`}
							>
								<span
									className={`absolute left-1 top-1 grid size-5 place-items-center rounded-full bg-slate-50 shadow-[0_2px_5px_rgba(62,70,88,.28)] transition-transform duration-200 ${item.connected ? 'translate-x-5' : 'translate-x-0'}`}
								>
									{busy === item.id && (
										<LoaderCircle
											size={12}
											className='animate-spin text-violet-600'
											aria-hidden='true'
										/>
									)}
								</span>
							</button>
						) : (
							<button
								type='button'
								aria-label={`Set up ${item.name}`}
								disabled={busy === item.id}
								onClick={() => void run(item.id, client.setup(item.id))}
								className='rounded-xl bg-violet-600 px-4 py-2.5 text-sm font-semibold text-white shadow-[0_8px_18px_rgba(104,69,216,.18)] transition-colors hover:bg-violet-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-violet-500 disabled:cursor-wait disabled:opacity-50'
							>
								{busy === item.id ? 'Setting up…' : 'Set up'}
							</button>
						)}
					</div>
				))}
			</div>
			{error && (
				<div
					role='alert'
					className='mt-4 flex items-start gap-3 rounded-xl border border-red-300/70 bg-red-50/80 p-4 text-sm text-red-700'
				>
					<CircleAlert size={18} className='mt-0.5 shrink-0' aria-hidden='true' />
					<p>{error}</p>
				</div>
			)}
		</section>
	)
}

function VpnSkeleton() {
	return (
		<div
			aria-label='Loading VPN providers'
			className='flex min-h-24 animate-pulse items-center gap-4 rounded-2xl border border-white/80 bg-white/48 p-5 shadow-[7px_7px_15px_rgba(126,137,156,.12),-7px_-7px_15px_rgba(255,255,255,.72)]'
		>
			<span className='size-11 rounded-xl bg-slate-200/80' />
			<div className='flex-1'>
				<div className='h-3.5 w-24 rounded-full bg-slate-200/85' />
				<div className='mt-2.5 h-3 w-16 rounded-full bg-slate-200/65' />
			</div>
			<span className='h-7 w-12 rounded-full bg-slate-200/80' />
		</div>
	)
}
