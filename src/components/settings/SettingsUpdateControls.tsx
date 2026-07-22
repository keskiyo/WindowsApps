import { ExternalLink, Github, RefreshCw, Send } from 'lucide-react'
import type { UpdateCheckStatus, UpdaterState } from '../../hooks/useUpdater'

interface Props {
	updater: UpdaterState
	onOpenGithub(): Promise<void>
	onOpenTelegram(): Promise<void>
}

export function SettingsUpdateControls({
	updater,
	onOpenGithub,
	onOpenTelegram,
}: Props) {
	return (
		<>
			<div className='flex items-center gap-4 border-b border-slate-200 p-5'>
				<span className='grid size-10 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
					<Github size={19} aria-hidden='true' />
				</span>
				<div className='min-w-0 flex-1'>
					<h2 className='font-medium'>Updates and source</h2>
					<p className='mt-1 text-sm text-slate-600'>
						{updater.update
							? `Version ${updater.update.version} is available.`
							: updateStatusText(updater.status)}
					</p>
				</div>
				<div className='flex shrink-0 flex-wrap justify-end gap-2'>
					<button
						type='button'
						disabled={updater.status === 'checking'}
						onClick={() => void updater.checkNow()}
						className='inline-flex items-center gap-2 rounded-xl bg-violet-600 px-3.5 py-2 text-sm font-medium text-white shadow-[0_8px_18px_rgba(104,69,216,.18)] hover:bg-violet-500 focus-visible:outline-2 focus-visible:outline-violet-500 disabled:opacity-50'
					>
						<RefreshCw
							size={16}
							className={updater.status === 'checking' ? 'animate-spin' : ''}
							aria-hidden='true'
						/>
						Check updates
					</button>
					<button
						type='button'
						aria-label='Open Windows Apps on GitHub'
						onClick={() => void onOpenGithub()}
						className='inline-flex items-center gap-2 rounded-xl border border-slate-300/70 px-3.5 py-2 text-sm font-medium text-slate-200 hover:bg-white/8 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<Github size={16} aria-hidden='true' />
						keskiyo
					</button>
				</div>
			</div>
			<button
				type='button'
				aria-label='Open @keskiyo on Telegram'
				onClick={() => void onOpenTelegram()}
				className='flex w-full items-center gap-4 p-5 text-left hover:bg-violet-100/55 focus-visible:outline-2 focus-visible:outline-violet-500'
			>
				<span className='grid size-10 place-items-center rounded-xl bg-[#229ED9]/15 text-[#5cc8f5]'>
					<Send size={19} aria-hidden='true' />
				</span>
				<span className='flex-1'>
					<span className='block font-medium'>Telegram</span>
					<span className='mt-1 block text-sm text-slate-600'>@keskiyo</span>
				</span>
				<ExternalLink size={17} className='text-slate-500' aria-hidden='true' />
			</button>
		</>
	)
}

function updateStatusText(status: UpdateCheckStatus): string {
	switch (status) {
		case 'checking':
			return 'Checking for updates...'
		case 'current':
			return 'You are running the latest version.'
		case 'available':
			return 'Update available.'
		case 'error':
			return 'Could not check for updates.'
		default:
			return 'Check for updates or open the project repository.'
	}
}
