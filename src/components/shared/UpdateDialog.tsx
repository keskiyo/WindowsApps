import { Download, Sparkles, X } from 'lucide-react'
import { useEffect, useRef } from 'react'

interface Props {
	version: string
	notes: string | null
	installing: boolean
	progress: number | null
	onInstall(): void
	onDismiss(): void
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
	notes,
	installing,
	progress,
	onInstall,
	onDismiss,
}: Props) {
	const highlights = releaseHighlights(notes)
	const dialogRef = useRef<HTMLElement>(null)
	const closeButtonRef = useRef<HTMLButtonElement>(null)
	const installLabel = installing
		? `Installing... ${progress ?? 0}%`
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
				className='update-modal-panel relative flex max-h-[calc(100vh-3rem)] w-full max-w-xl flex-col overflow-hidden rounded-2xl border border-violet-300/35 bg-[#363940] text-slate-100 shadow-[0_24px_70px_rgba(0,0,0,0.45),0_0_38px_rgba(167,139,250,0.18)]'
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

					{installing && (
						<div className='mt-5' aria-label='Update progress'>
							<div className='h-2 overflow-hidden rounded-full bg-slate-900/50'>
								<div
									className='h-full rounded-full bg-violet-400 transition-[width]'
									style={{ width: `${progress ?? 0}%` }}
								/>
							</div>
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
						<Download size={16} aria-hidden='true' />
						{installLabel}
					</button>
				</div>
			</section>
		</div>
	)
}
