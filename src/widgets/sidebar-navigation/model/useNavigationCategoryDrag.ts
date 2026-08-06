import type { DragEndEvent, DragStartEvent } from '@dnd-kit/core'
import { useEffect, useRef, useState } from 'react'
import { getDropAction } from '../../../entities/app'
import type { AppCategory } from '../../../entities/category'
import type { NavigationCategoryDragOptions } from '../types'

function categoryFromDrag(event: DragStartEvent) {
	const data = event.active.data.current
	return data?.type === 'category-sort' && typeof data.category === 'string'
		? (data.category as AppCategory)
		: null
}

function isPointerInside(event: PointerEvent, navigation: HTMLElement) {
	const bounds = navigation.getBoundingClientRect()
	return (
		event.clientX >= bounds.left &&
		event.clientX <= bounds.right &&
		event.clientY >= bounds.top &&
		event.clientY <= bounds.bottom
	)
}

export function useNavigationCategoryDrag({
	navigationRef,
	onReorderCategory,
}: NavigationCategoryDragOptions) {
	const [activeCategory, setActiveCategory] = useState<AppCategory | null>(
		null,
	)
	const activeCategoryRef = useRef<AppCategory | null>(null)
	const cancelled = useRef(false)
	const pointerListener = useRef<((event: PointerEvent) => void) | null>(null)
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
	function setActiveDragCategory(category: AppCategory | null) {
		activeCategoryRef.current = category
		setActiveCategory(category)
	}
	function handleDragStart(event: DragStartEvent) {
		stopPointerListener()
		cancelled.current = false
		setActiveDragCategory(categoryFromDrag(event))
		if ('clientX' in event.activatorEvent) {
			const listener = (pointer: PointerEvent) => {
				if (
					activeCategoryRef.current &&
					navigationRef.current &&
					!isPointerInside(pointer, navigationRef.current)
				) {
					cancelled.current = true
					setActiveDragCategory(null)
					window.removeEventListener('pointermove', listener, true)
					pointerListener.current = null
				}
			}
			pointerListener.current = listener
			window.addEventListener('pointermove', listener, true)
		}
	}
	function clearDrag() {
		stopPointerListener()
		cancelled.current = false
		setActiveDragCategory(null)
	}
	function handleDragEnd(event: DragEndEvent) {
		if (!cancelled.current) {
			const action = getDropAction(
				event.active.data.current,
				event.over?.data.current,
			)
			if (action?.type === 'reorder-category') {
				onReorderCategory(action.active, action.over)
			}
		}
		clearDrag()
	}
	return {
		activeCategory,
		cancelDrop: () => cancelled.current,
		handleDragStart,
		handleDragEnd,
		handleDragCancel: clearDrag,
	}
}
