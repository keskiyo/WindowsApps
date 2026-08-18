import { useEffect, type RefObject } from 'react'
import { useBodyScrollLock } from './useBodyScrollLock'
import { useFocusTrap } from './useFocusTrap'

interface ModalDialogOptions {
	ref: RefObject<HTMLElement | null>
	initialFocusRef?: RefObject<HTMLElement | null>
	restoreFocusTo?: RefObject<HTMLElement | null>
	onDismiss?: () => void
	dismissible?: boolean
}

export function useModalDialog({
	ref,
	initialFocusRef,
	restoreFocusTo,
	onDismiss,
	dismissible = true,
}: ModalDialogOptions) {
	useBodyScrollLock()
	useFocusTrap(ref)

	useEffect(() => {
		const opener = document.activeElement
		const requested = restoreFocusTo?.current ?? null
		const target =
			initialFocusRef?.current ??
			ref.current?.querySelector<HTMLElement>('button')
		target?.focus()
		return () => {
			const restored = requested ?? opener
			if (restored instanceof HTMLElement && restored.isConnected)
				restored.focus()
		}
	}, [initialFocusRef, ref, restoreFocusTo])

	useEffect(() => {
		if (!onDismiss || !dismissible) return
		function onKeyDown(event: KeyboardEvent) {
			if (event.key !== 'Escape') return
			event.preventDefault()
			onDismiss?.()
		}
		document.addEventListener('keydown', onKeyDown)
		return () => document.removeEventListener('keydown', onKeyDown)
	}, [dismissible, onDismiss])
}
