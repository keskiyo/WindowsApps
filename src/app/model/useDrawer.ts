import { useCallback, useEffect, useState } from 'react'

export function useDrawer(desktopNavigation: boolean) {
	const [open, setOpen] = useState(false)
	const [mounted, setMounted] = useState(false)
	const animate = import.meta.env.MODE !== 'test'

	const close = useCallback(() => {
		setOpen(false)
		if (!animate) setMounted(false)
	}, [animate])

	useEffect(() => {
		if (desktopNavigation) setOpen(false)
	}, [desktopNavigation])

	useEffect(() => {
		if (open) setMounted(true)
	}, [open])

	return {
		open,
		mounted,
		close,
		onOpen: () => setOpen(true),
		onExited: () => setMounted(false),
	}
}
