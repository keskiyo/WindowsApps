import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { AppCard } from '../../../../src/components/apps/AppCard/AppCard'
import type { AppCategory, AppInfo, CategoryDefinition } from '../../../../src/types'

vi.mock('@dnd-kit/core', () => ({
	useDraggable: () => ({
		attributes: {},
		listeners: {},
		setActivatorNodeRef: vi.fn(),
		setNodeRef: vi.fn(),
		transform: { x: 240, y: 32, scaleX: 1, scaleY: 1 },
		isDragging: true,
	}),
}))

vi.mock('../../../../src/store/useIsLaunching', () => ({
	useIsLaunching: () => false,
}))

const development: CategoryDefinition = {
	id: 'development',
	label: 'Development',
	builtIn: true,
}

const app: AppInfo = {
	id: 'claude',
	name: 'Claude',
	path: 'C:\\Claude\\claude.exe',
	iconBase64: null,
	category: 'development',
	launchKind: 'executable',
	sourceKind: 'registry',
	description: null,
	version: null,
	publisher: null,
	installLocation: null,
	canUninstall: false,
}

describe('AppCard', () => {
	it('keeps an active source in the grid without a transform', () => {
		render(
			<AppCard
				app={app}
				isFavorite={false}
				categories={[development]}
				categoryOrder={['development'] as AppCategory[]}
				onToggleFavorite={vi.fn()}
				onLaunch={vi.fn().mockResolvedValue(undefined)}
				onMove={vi.fn()}
				onInfo={vi.fn()}
				onUninstall={vi.fn()}
				onHide={vi.fn()}
				onRestore={vi.fn()}
				onDemote={vi.fn()}
			/>,
		)

		const source = screen.getByRole('button', { name: 'Launch Claude' })
			.parentElement
		expect(source?.style.transform).toBe('')
	})
})
