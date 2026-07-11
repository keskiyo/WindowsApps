import {
	AlertTriangle,
	Download,
	ExternalLink,
	LoaderCircle,
	RefreshCw,
	Sparkles,
	X,
} from 'lucide-react'
import { useEffect, useRef } from 'react'
import type { UpdateInstallPhase } from '../../hooks/useUpdater'

interface Props {
	version: string
	date: string | null
	packageSize: number | null
	releaseUrl: string | null
	notes: string | null
	installing: boolean
	progress: number | null
	downloadedBytes: number
	totalBytes: number | null
	phase: UpdateInstallPhase
	error: string | null
	onInstall(): void
	onDismiss(): void
	onOpenRelease(): void
}

const UPDATE_STEPS: Exclude<UpdateInstallPhase, 'idle' | 'failed'>[] = [
	'downloading',
	'verifying',
	'installing',
	'restarting',
]

function formatBytes(bytes: number): string {
	return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function formatReleaseDate(value: string): string | null {
	const date = new Date(value)
	if (Number.isNaN(date.getTime())) return null
	return new Intl.DateTimeFormat('en-GB', {
		day: '2-digit',
		month: 'short',
		year: 'numeric',
		timeZone: 'UTC',
	}).format(date)
}

function cleanMarkdownLine(value: string): string {
	return value
		.replace(/^\s*[-*]\s+/, '')
		.replace(/\[([^\]]+)]\([^)]+\)/g, '$1')
		.replace(/[`*_>#]/g, '')
		.replace(/\s+/g, ' ')
		.trim()
}

function truncateLine(value: string): string {
	return value.length > 180 ? `${value.slice(0, 177).trim()}...` : value
}

export function releaseHighlights(notes: string | null): string[] {
	if (!notes) return []

	const lines = notes.replace(/\r\n/g, '\n').split('\n')
	const highlightHeadingIndex = lines.findIndex(line =>
		/^#{0,6}\s*highlights\s*$/i.test(line.trim()),
	)
	const releaseSectionLines =
		highlightHeadingIndex >= 0 ? lines.slice(highlightHeadingIndex + 1) : lines
	const nextHeadingIndex = releaseSectionLines.findIndex(line =>
		/^#{1,6}\s+\S/.test(line.trim()),
	)
	const searchLines =
		highlightHeadingIndex >= 0 && nextHeadingIndex >= 0
			? releaseSectionLines.slice(0, nextHeadingIndex)
			: releaseSectionLines
	const bullets = searchLines
		.filter(line => /^\s*[-*]\s+/.test(line))
		.map(line => truncateLine(cleanMarkdownLine(line)))
		.filter(Boolean)

	if (bullets.length) return bullets.slice(0, 4)

	return searchLines
		.map(cleanMarkdownLine)
		.filter(Boolean)
		.slice(0, 3)
}

export function UpdateDialog({
	version,
	date,
	packageSize,
	releaseUrl,
	notes,
	installing,
	progress,
	downloadedBytes,
	totalBytes,
	phase,
	error,
	onInstall,
	onDismiss,
	onOpenRelease,
}: Props) {
	const highlights = releaseHighlights(notes)
	const releaseDate = date ? formatReleaseDate(date) : null
	const effectiveTotal = totalBytes ?? packageSize
	const activeStep = UPDATE_STEPS.indexOf(
		phase as Exclude<UpdateInstallPhase, 'idle' | 'failed'>,
	)
	const dialogRef = useRef<HTMLElement>(null)
	const closeButtonRef = useRef<HTMLButtonElement>(null)
	const installLabel =
		phase === 'failed'
			? 'Retry update'
			: phase === 'downloading'
				? `Downloading... ${progress ?? 0}%`
				: phase === 'verifying'
					? 'Verifying...'
				: phase === 'installing'
					? 'Installing...'
					: phase === 'restarting'
						? 'Restarting...'
					: 'Update & restart'

	useEffect(() => {
		const previousFocus = document.activeElement
		closeButtonRef.current?.focus()

		function focusableElements() {
			return Array.from(
				dialogRef.current?.querySelectorAll<HTMLElement>(
					'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
				) ?? [],
			).filter(element => !element.hasAttribute('aria-hidden'))
		}

		function onKeyDown(event: KeyboardEvent) {
			if (event.key === 'Escape' && !installing) {
				event.preventDefault()
				onDismiss()
				return
			}
			if (event.key !== 'Tab') return

			const elements = focusableElements()
			if (!elements.length) return
			const first = elements[0]
			const last = elements[elements.length - 1]
			if (event.shiftKey && document.activeElement === first) {
				event.preventDefault()
				last.focus()
			} else if (!event.shiftKey && document.activeElement === last) {
				event.preventDefault()
				first.focus()
			}
		}

		document.addEventListener('keydown', onKeyDown)
		return () => {
			document.removeEventListener('keydown', onKeyDown)
			if (previousFocus instanceof HTMLElement) previousFocus.focus()
		}
	}, [installing, onDismiss])

	return (
		<div className='update-modal-backdrop fixed inset-0 z-[500] grid place-items-center bg-slate-950/55 px-4 py-6 backdrop-blur-sm'>
			<section
				ref={dialogRef}
				role='dialog'
				aria-modal='true'
				aria-labelledby='update-dialog-title'
				aria-describedby='update-dialog-description'
				className='update-modal-panel relative flex min-h-[31rem] max-h-[calc(100vh-3rem)] w-full max-w-xl flex-col overflow-hidden rounded-lg border border-violet-300/35 bg-[#363940] text-slate-100 shadow-[0_24px_70px_rgba(0,0,0,0.45),0_0_38px_rgba(167,139,250,0.18)]'
			>
				<div className='flex items-start gap-4 border-b border-white/10 px-6 py-5'>
					<div className='grid size-11 shrink-0 place-items-center rounded-xl border border-violet-300/25 bg-violet-500/18 text-violet-200'>
						<Sparkles size={20} aria-hidden='true' />
					</div>
					<div className='min-w-0 flex-1'>
						<h2
							id='update-dialog-title'
							className='text-lg font-semibold leading-6 text-slate-50'
						>
							Update {version} available
						</h2>
						<p className='mt-1 text-sm leading-6 text-slate-300'>
						<span id='update-dialog-description'>
							Review the release highlights before installing.
						</span>
						</p>
						{(releaseDate || packageSize) && (
							<div className='mt-2 flex flex-wrap items-center gap-2 text-xs text-slate-400'>
								{releaseDate && <span>{releaseDate}</span>}
								{releaseDate && packageSize && <span aria-hidden='true'>•</span>}
								{packageSize && <span>{formatBytes(packageSize)}</span>}
							</div>
						)}
					</div>
					<button
						ref={closeButtonRef}
						type='button'
						aria-label='Dismiss update'
						onClick={onDismiss}
						disabled={installing}
						className='grid size-9 shrink-0 place-items-center rounded-lg text-slate-300 hover:bg-white/10 hover:text-white focus-visible:outline-2 focus-visible:outline-violet-300 disabled:opacity-40'
					>
						<X size={17} aria-hidden='true' />
					</button>
				</div>

				<div className='min-h-0 overflow-y-auto px-6 py-5'>
					<h3 className='text-sm font-semibold uppercase tracking-[0.12em] text-violet-200'>
						Highlights
					</h3>
					{highlights.length ? (
						<ul className='mt-4 space-y-3 text-sm leading-6 text-slate-200'>
							{highlights.map(item => (
								<li key={item} className='flex gap-3'>
									<span
										className='mt-2 size-1.5 shrink-0 rounded-full bg-violet-300'
										aria-hidden='true'
									/>
									<span>{item}</span>
								</li>
							))}
						</ul>
					) : (
						<p className='mt-4 rounded-xl border border-white/10 bg-white/5 px-4 py-3 text-sm leading-6 text-slate-300'>
							Release notes are available in the latest GitHub
							release.
						</p>
					)}

					{releaseUrl && (
						<a
							href={releaseUrl}
							onClick={event => {
								event.preventDefault()
								onOpenRelease()
							}}
							className='mt-4 inline-flex items-center gap-2 text-sm font-medium text-violet-200 hover:text-violet-100 focus-visible:outline-2 focus-visible:outline-violet-300'
						>
							View full release notes
							<ExternalLink size={14} aria-hidden='true' />
						</a>
					)}

					{installing && (
						<div className='mt-5' aria-label='Update progress' aria-live='polite'>
							<div className='mb-3 flex items-center justify-between gap-2 text-[11px] font-medium'>
								{UPDATE_STEPS.map((step, index) => (
									<span
										key={step}
										className={
											index <= activeStep ? 'text-violet-200' : 'text-slate-500'
										}
									>
										{step[0].toUpperCase() + step.slice(1)}
									</span>
								))}
							</div>
							<div className='h-2 overflow-hidden rounded-full bg-slate-900/50'>
								<div
									className='h-full rounded-full bg-violet-400 transition-[width]'
									style={{ width: `${progress ?? 0}%` }}
								/>
							</div>
							{phase === 'downloading' && effectiveTotal && (
								<div className='mt-2 flex justify-between text-xs text-slate-300'>
									<span
										aria-label={`${formatBytes(downloadedBytes)} of ${formatBytes(effectiveTotal)}`}
									>
										{formatBytes(downloadedBytes)} of {formatBytes(effectiveTotal)}
									</span>
									<span>{progress ?? 0}%</span>
								</div>
							)}
						</div>
					)}

					{phase === 'failed' && error && (
						<div
							role='alert'
							className='mt-5 flex gap-3 rounded-xl border border-rose-300/30 bg-rose-500/10 px-4 py-3 text-sm leading-6 text-rose-100'
						>
							<AlertTriangle className='mt-0.5 size-4 shrink-0' aria-hidden='true' />
							<span>{error}</span>
						</div>
					)}
				</div>

				<div className='flex flex-col-reverse gap-3 border-t border-white/10 px-6 py-5 sm:flex-row sm:justify-end'>
					<button
						type='button'
						onClick={onDismiss}
						disabled={installing}
						className='inline-flex h-10 items-center justify-center rounded-xl border border-white/12 bg-white/6 px-4 text-sm font-medium text-slate-200 hover:bg-white/10 focus-visible:outline-2 focus-visible:outline-violet-300 disabled:opacity-40'
					>
						Later
					</button>
					<button
						type='button'
						onClick={onInstall}
						disabled={installing}
						className='inline-flex h-10 items-center justify-center gap-2 rounded-xl bg-violet-600 px-4 text-sm font-semibold text-white shadow-lg shadow-violet-950/20 hover:bg-violet-500 focus-visible:outline-2 focus-visible:outline-violet-300 disabled:opacity-70'
					>
						{phase === 'failed' ? (
							<RefreshCw size={16} aria-hidden='true' />
						) : installing ? (
							<LoaderCircle className='animate-spin' size={16} aria-hidden='true' />
						) : (
							<Download size={16} aria-hidden='true' />
						)}
						{installLabel}
					</button>
				</div>
			</section>
		</div>
	)
}
