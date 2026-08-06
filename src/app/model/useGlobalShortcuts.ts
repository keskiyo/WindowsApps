import { useEffect } from 'react'

interface GlobalShortcuts {
	onToggleQuickLaunch: () => void
	onSearchFromShortcut: () => void
	onFocusSearch: () => void
}

function isTypingTarget(target: EventTarget | null): boolean {
	return (
		target instanceof HTMLInputElement ||
		target instanceof HTMLTextAreaElement ||
		(target as HTMLElement | null)?.isContentEditable === true
	)
}

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
