import { SettingsToggle } from '../components/SettingsToggle'
import { AppWindow, Keyboard, Power } from 'lucide-react'
import { SettingsUpdateControls } from './SettingsUpdateControls'
import type { GeneralSettingsProps } from '../../types'

export function GeneralSettings({
	settings,
	saving,
	updater,
	onToggleAutostart,
	onOpenGithub,
	onOpenTelegram,
	onOpenAppsSettings,
}: GeneralSettingsProps) {
	return (
		<>
			<div className='settings-surface mt-5 overflow-hidden rounded-2xl border border-white/85 bg-white/58'>
				<div className='flex items-center gap-4 border-b border-slate-200 p-5'>
					<span className='grid size-10 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
						<Power size={19} aria-hidden='true' />
					</span>
					<div className='min-w-0 flex-1'>
						<h2 className='font-medium'>Launch when Windows starts</h2>
						<p className='mt-1 text-sm text-slate-600'>
							Open Windows Apps automatically after you sign in.
						</p>
					</div>
					<SettingsToggle
						label='Launch when Windows starts'
						checked={settings?.autostartEnabled ?? false}
						disabled={!settings || saving}
						onToggle={() => void onToggleAutostart()}
					/>
				</div>
				<div className='flex items-center gap-4 border-b border-slate-200 p-5'>
					<span className='grid size-10 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
						<Keyboard size={19} aria-hidden='true' />
					</span>
					<div className='flex-1'>
						<h2 className='font-medium'>Global shortcut</h2>
						<p className='mt-1 text-sm text-slate-600'>
							Uses the physical Q key, independent of keyboard
							layout.
						</p>
					</div>
					<kbd className='rounded-lg border border-slate-300 bg-slate-100 px-3 py-1.5 text-sm text-slate-700'>
						{settings?.shortcut.label ?? 'Win+Shift+Q'}
					</kbd>
				</div>
				<div className='flex items-center gap-4 border-b border-slate-200 p-5'>
					<span className='grid size-10 place-items-center rounded-xl bg-slate-200/70 text-violet-700 shadow-inner'>
						<AppWindow size={19} aria-hidden='true' />
					</span>
					<div className='min-w-0 flex-1'>
						<h2 className='font-medium'>Windows installed apps</h2>
						<p className='mt-1 text-sm text-slate-600'>
							Open the Windows Settings page to add or remove
							programs.
						</p>
					</div>
					<button
						type='button'
						aria-label='Open Windows installed apps'
						onClick={() => void onOpenAppsSettings()}
						className='utility-accent-button rounded-lg px-4 py-2 text-sm font-medium text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-violet-500'
					>
						Open
					</button>
				</div>
				<SettingsUpdateControls
					updater={updater}
					onOpenGithub={onOpenGithub}
					onOpenTelegram={onOpenTelegram}
				/>
			</div>
			{settings?.shortcut.error && (
				<p className='mt-4 text-sm text-amber-700'>
					{settings.shortcut.error}
				</p>
			)}
		</>
	)
}
