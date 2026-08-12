import { usePreferencesBackup } from './usePreferencesBackup'
import type { PreferencesBackupProps } from './types'
import { ACTION_ROW } from '../../../data'

export function PreferencesBackup(props: PreferencesBackupProps) {
	const {
		inputRef,
		pending,
		exporting,
		message,
		error,
		exportSettings,
		selectImport,
		confirmPending,
		clearFeedback,
		setPending,
	} = usePreferencesBackup(props)

	return (
		<section className="settings-surface rounded-2xl border border-white/85 bg-white/58 p-5">
			<h2 className="font-medium">Backup &amp; restore</h2>
			<p className="mt-1 max-w-2xl text-sm leading-6 text-slate-600">
				Export categories, Favorites, Hidden apps, and scenarios. A local
				fallback copy is created before each saved change.
			</p>
			<div className={`mt-4 ${ACTION_ROW}`}>
				<button
					type="button"
					disabled={exporting}
					onClick={() => void exportSettings()}
					className="utility-accent-button rounded-lg px-4 py-2 text-sm font-medium text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-violet-500"
				>
					{exporting ? 'Saving\u2026' : 'Export settings'}
				</button>
				<button
					type="button"
					onClick={() => inputRef.current?.click()}
					className="rounded-lg border border-slate-300 bg-slate-100 px-4 py-2 text-sm font-medium text-slate-700 hover:border-violet-400/45 hover:bg-violet-100/75 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-violet-500"
				>
					Import settings
				</button>
				<button
					type="button"
					onClick={() => {
						clearFeedback()
						setPending({ kind: 'restore' })
					}}
					className="rounded-lg border border-slate-300 bg-slate-100 px-4 py-2 text-sm font-medium text-slate-700 hover:border-violet-400/45 hover:bg-violet-100/75 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-violet-500"
				>
					Restore local backup
				</button>
			</div>
			<input
				ref={inputRef}
				type="file"
				accept="application/json,.json"
				aria-label="Choose settings backup"
				onChange={event => void selectImport(event)}
				className="sr-only"
			/>
			{pending && (
				<div className="mt-4 flex flex-col gap-3 rounded-xl border border-violet-400/35 bg-violet-500/8 p-4 sm:flex-row sm:items-center">
					<p className="min-w-0 text-sm leading-6 text-slate-700 sm:flex-1">
						{pending.kind === 'import'
							? `Import ${pending.name}? This replaces your current settings.`
							: 'Restore the local backup? This replaces your current settings.'}
					</p>
					<div className="flex gap-3 sm:shrink-0">
						<button
							type="button"
							onClick={() => setPending(null)}
							className="rounded-lg px-3 py-2 text-sm font-medium text-slate-600 hover:bg-slate-200/70 focus-visible:outline-2 focus-visible:outline-violet-500"
						>
							Cancel
						</button>
						<button
							type="button"
							onClick={confirmPending}
							className="utility-accent-button rounded-lg px-4 py-2 text-sm font-medium text-white focus-visible:outline-2 focus-visible:outline-violet-500"
						>
							{pending.kind === 'import' ? 'Import backup' : 'Restore backup'}
						</button>
					</div>
				</div>
			)}
			{message && (
				<p role="status" className="mt-4 text-sm text-emerald-700">
					{message}
				</p>
			)}
			{error && (
				<p role="alert" className="mt-4 text-sm text-red-700">
					{error}
				</p>
			)}
		</section>
	)
}
