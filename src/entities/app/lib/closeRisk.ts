import type { AppInfo } from '../model/app.types'

const CRITICAL = 'close.critical'
const SESSION = 'close.session'
const NOT_CLOSABLE = 'close.not_closable'

const WARNINGS: Record<string, string> = {
	[SESSION]:
		'This is part of the Windows desktop. Closing it ends the shell — the taskbar, the Start menu and open folders go with it, and Windows restarts it.',
	[NOT_CLOSABLE]:
		'This entry launches through Windows rather than an executable, so a close list has no process to end and would do nothing.',
}

const BLOCKED_MESSAGE =
	'Windows cannot survive closing this process, so it cannot go in a close list.'

const NOT_CLOSABLE_MESSAGE =
	'There is no process to close here: this entry is a Windows shell location, not an executable.'

export interface CloseRiskBadge {
	label: string
	tone: 'danger' | 'muted'
}

const BADGES: Record<string, CloseRiskBadge> = {
	[CRITICAL]: { label: 'Blocked', tone: 'danger' },
	[SESSION]: { label: 'Danger', tone: 'danger' },
	[NOT_CLOSABLE]: { label: 'Cannot close', tone: 'muted' },
}

type CloseCandidate = Pick<AppInfo, 'closeRisk'>

export function isCloseBlocked(app: CloseCandidate): boolean {
	return (
		app.closeRisk === CRITICAL ||
		app.closeRisk === SESSION ||
		app.closeRisk === NOT_CLOSABLE
	)
}

export function closeBlockedMessage(app: CloseCandidate): string {
	return app.closeRisk === NOT_CLOSABLE
		? NOT_CLOSABLE_MESSAGE
		: BLOCKED_MESSAGE
}

export function closeRiskBadge(app: CloseCandidate): CloseRiskBadge | null {
	const risk = app.closeRisk
	if (!risk) return null
	return BADGES[risk] ?? { label: 'Danger', tone: 'danger' }
}

export function closeRiskReason(app: CloseCandidate): string | null {
	if (app.closeRisk === CRITICAL) return BLOCKED_MESSAGE
	if (app.closeRisk === NOT_CLOSABLE) return NOT_CLOSABLE_MESSAGE
	return closeRiskWarning(app)
}

export function closeRiskWarning(app: CloseCandidate): string | null {
	const risk = app.closeRisk
	if (!risk || risk === CRITICAL) return null
	return (
		WARNINGS[risk] ??
		'Windows reports this as a system component. Closing it may disrupt the desktop.'
	)
}
