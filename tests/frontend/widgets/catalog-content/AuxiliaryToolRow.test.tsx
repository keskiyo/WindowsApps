import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { AuxiliaryToolRow } from '../../../../src/widgets/catalog-content/ui/AuxiliaryToolRow/AuxiliaryToolRow'
import type { AppInfo } from '../../../../src/entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../src/entities/category'

const draggable = vi.hoisted(() => ({
	setNodeRef: vi.fn(),
	setActivatorNodeRef: vi.fn(),
	onPointerDown: vi.fn(),
}))

vi.mock('@dnd-kit/core', () => ({
	useDraggable: () => ({
		attributes: {},
		listeners: { onPointerDown: draggable.onPointerDown },
		setNodeRef: draggable.setNodeRef,
		setActivatorNodeRef: draggable.setActivatorNodeRef,
		transform: null,
		isDragging: false,
	}),
}))

vi.mock('../../../../src/features/launch-app/model/useIsLaunching', () => ({
	useIsLaunching: () => false,
}))

const development: CategoryDefinition = {
	id: 'development',
	label: 'Development',
	builtIn: true,
}

const app: AppInfo = {
	id: 'claude-code',
	name: 'Claude Code',
	path: 'C:\\Tools\\claude.exe',
	iconBase64: null,
	category: 'development',
	launchKind: 'executable',
	sourceKind: 'registry',
	description: null,
	version: 'v2.1.186.0',
	publisher: 'Anthropic PBC',
	installLocation: null,
	canUninstall: false,
}

function props(appOverride: AppInfo = app) {
	return {
		app: appOverride,
		categories: [development],
		categoryOrder: ['development'] as AppCategory[],
		onLaunch: vi.fn().mockResolvedValue(undefined),
		onMove: vi.fn(),
		onInfo: vi.fn(),
		onUninstall: vi.fn(),
		onRestore: vi.fn(),
		onDemote: vi.fn(),
	}
}

beforeEach(() => {
	draggable.setNodeRef.mockReset()
	draggable.setActivatorNodeRef.mockReset()
	draggable.onPointerDown.mockReset()
})

describe('AuxiliaryToolRow', () => {
	it('shows compact tool identity and accessible actions', () => {
		render(<AuxiliaryToolRow {...props()} />)

		expect(
			screen.getByRole('button', { name: 'Launch Claude Code' }),
		).toHaveTextContent('Claude Code')
		expect(
			screen.getByText('Anthropic PBC · v2.1.186.0'),
		).toBeInTheDocument()
		expect(
			screen.getByRole('button', { name: 'Manage Claude Code' }),
		).toHaveAttribute('aria-haspopup', 'menu')
	})

	it('omits empty metadata without leaving a separator', () => {
		render(
			<AuxiliaryToolRow
				{...props({ ...app, publisher: null, version: null })}
			/>,
		)

		expect(screen.queryByText('·')).not.toBeInTheDocument()
		expect(screen.queryByText('Unknown publisher')).not.toBeInTheDocument()
	})

	it('launches the selected tool', async () => {
		const rowProps = props()
		render(<AuxiliaryToolRow {...rowProps} />)

		await userEvent.click(
			screen.getByRole('button', { name: 'Launch Claude Code' }),
		)

		expect(rowProps.onLaunch).toHaveBeenCalledOnce()
		expect(rowProps.onLaunch).toHaveBeenCalledWith(app)
	})
})
