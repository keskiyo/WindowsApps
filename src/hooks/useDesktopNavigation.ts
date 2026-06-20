import { useEffect, useState } from 'react'

const QUERY = '(min-width: 1024px)'

export function useDesktopNavigation(): boolean {
	const [desktop, setDesktop] = useState(
		() => globalThis.matchMedia?.(QUERY).matches ?? false,
	)
	useEffect(() => {
		const media = globalThis.matchMedia?.(QUERY)
		if (!media) return
		const update = () => setDesktop(media.matches)
		update()
		media.addEventListener('change', update)
		return () => media.removeEventListener('change', update)
	}, [])
	return desktop
}
