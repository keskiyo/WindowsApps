import { useEffect } from 'react'

interface GlobalShortcuts {
	onToggleQuickLaunch: () => void
	/** Ctrl+F: the user is retyping a query, so the existing text is replaced. */
	onSearchFromShortcut: () => void
	/** "/": a bare jump to the field that leaves any existing query alone. */
	onFocusSearch: () => void
}

function isTypingTarget(target: EventTarget | null): boolean {
	return (
		target instanceof HTMLInputElement ||
		target instanceof HTMLTextAreaElement ||
		(target as HTMLElement | null)?.isContentEditable === true
	)
}

/**
 * Global keyboard shortcuts: Ctrl+K opens the quick-launch palette; Ctrl+F or "/"
 * jump to the search field (a launcher should be keyboard-first).
 *
 * Each combination is matched on the physical `event.code` as well as `event.key`, because on a
 * non-Latin keyboard layout `key` carries the translated character and the shortcut would
 * otherwise stop working for exactly the users who most need the palette. Ctrl+P is swallowed
 * so the webview never opens a print dialog over the app.
 */
export function useGlobalShortcuts({
	onToggleQuickLaunch,
	onSearchFromShortcut,
	onFocusSearch,
}: GlobalShortcuts) {
	useEffect(() => {
		function onKeyDown(event: KeyboardEvent) {
			const typing = isTypingTarget(event.target)
			const commandOrControl = event.ctrlKey || event.metaKey
			const isQuickLaunchShortcut =
				commandOrControl &&
				(event.code === 'KeyK' || event.key.toLowerCase() === 'k')
			const isSearchShortcut =
				commandOrControl &&
				(event.code === 'KeyF' || event.key.toLowerCase() === 'f')
			const isPrintShortcut =
				commandOrControl &&
				(event.code === 'KeyP' || event.key.toLowerCase() === 'p')
			if (isPrintShortcut) {
				event.preventDefault()
				event.stopPropagation()
				return
			}
			if (isQuickLaunchShortcut) {
				event.preventDefault()
				event.stopPropagation()
				onToggleQuickLaunch()
				return
			}
			if (isSearchShortcut) {
				event.preventDefault()
				event.stopPropagation()
				onSearchFromShortcut()
				return
			}
			if (event.key === '/' && !typing) {
				event.preventDefault()
				onFocusSearch()
			}
		}
		document.addEventListener('keydown', onKeyDown, { capture: true })
		return () =>
			document.removeEventListener('keydown', onKeyDown, {
				capture: true,
			})
	}, [onToggleQuickLaunch, onSearchFromShortcut, onFocusSearch])
}
