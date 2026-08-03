import type {
	DragEndEvent,
	DragStartEvent,
} from '@dnd-kit/core'
import { useEffect, useRef, useState, type RefObject } from 'react'
import { getDropAction, type DragData } from '../../../../entities/app'
import type { AppCategory } from '../../../../entities/category'

export type CatalogDragPreview =
	| { type: 'app'; appId: string }
	| null

interface Props {
	listRef: RefObject<HTMLDivElement>
	onMoveApp(id: string, category: AppCategory): void
}

function previewFromDrag(event: DragStartEvent): CatalogDragPreview {
	const data = event.active.data.current as DragData | undefined
	if (data?.type === 'app' && typeof data.appId === 'string') {
		return { type: 'app', appId: data.appId }
	}
	return null
}

function isPointerInside(event: PointerEvent, list: HTMLDivElement) {
	const boundary =
		(list.closest('#catalog-scroll') as HTMLElement | null) ?? list
	const bounds = boundary.getBoundingClientRect()
	return (
		event.clientX >= bounds.left &&
		event.clientX <= bounds.right &&
		event.clientY >= bounds.top &&
		event.clientY <= bounds.bottom
	)
}

export function useCatalogDrag({
	listRef,
	onMoveApp,
}: Props) {
	const [preview, setPreviewState] = useState<CatalogDragPreview>(null)
	const activePreview = useRef<CatalogDragPreview>(null)
	const cancelled = useRef(false)
	const pointerListener = useRef<((event: PointerEvent) => void) | null>(
		null,
	)
	function stopPointerListener() {
		if (pointerListener.current) {
			window.removeEventListener(
				'pointermove',
				pointerListener.current,
				true,
			)
			pointerListener.current = null
		}
	}
	useEffect(() => stopPointerListener, [])
	function setPreview(next: CatalogDragPreview) {
		activePreview.current = next
		setPreviewState(next)
	}
	function clearDrag() {
		stopPointerListener()
		cancelled.current = false
		setPreview(null)
	}
	function handleDragStart(event: DragStartEvent) {
		stopPointerListener()
		cancelled.current = false
		setPreview(previewFromDrag(event))
		if ('clientX' in event.activatorEvent) {
			const listener = (pointer: PointerEvent) => {
				if (
					activePreview.current &&
					listRef.current &&
					!isPointerInside(pointer, listRef.current)
				) {
					cancelled.current = true
					setPreview(null)
					window.removeEventListener('pointermove', listener, true)
					pointerListener.current = null
				}
			}
			pointerListener.current = listener
			window.addEventListener('pointermove', listener, true)
		}
	}
	function handleDragEnd(event: DragEndEvent) {
		if (!cancelled.current) {
			const action = getDropAction(
				event.active.data.current as DragData | undefined,
				event.over?.data.current as DragData | undefined,
			)
			if (action?.type === 'move-app') {
				onMoveApp(action.appId, action.category)
			}
		}
		clearDrag()
	}
	return {
		preview,
		cancelDrop: () => cancelled.current,
		handleDragStart,
		handleDragEnd,
		handleDragCancel: clearDrag,
	}
}
