export const ACTION_BUTTON =
	'inline-flex min-h-11 items-center justify-center gap-2 rounded-xl px-4 py-2.5 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 disabled:opacity-50'

export const CONFIRM_BUTTON =
	'inline-flex min-h-10 items-center justify-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 disabled:opacity-50'

export const CANCEL_BUTTON = `${CONFIRM_BUTTON} border border-slate-300/80 bg-white/60 text-slate-700 hover:bg-violet-100/70 focus-visible:outline-violet-400`

export const ACTION_ROW = 'grid gap-2 sm:flex sm:flex-wrap sm:justify-end'

export const UNINSTALL_METHOD_LABELS = {
	registered_command: 'Registered command',
	msi: 'MSI',
	msix: 'MSIX',
} as const

export const UNINSTALL_RESULT_LABELS = {
	succeeded: 'Succeeded',
	failed: 'Failed',
} as const

export const timestampFormatter = new Intl.DateTimeFormat(undefined, {
	dateStyle: 'medium',
	timeStyle: 'short',
})

export const SOURCE_STATE_LABELS = {
	never_run: 'Never run',
	fresh: 'Up to date',
	stale: 'Serving older data',
	incomplete: 'Stopped early',
	failed_without_snapshot: 'Never succeeded',
	unknown: 'Unknown',
} as const

export const SOURCE_ERROR_LABELS = {
	cancelled: 'Cancelled',
	timed_out: 'Timed out',
	entry_limit: 'Entry limit reached',
	provider_failed: 'Did not answer',
	unknown: 'Unknown',
} as const
