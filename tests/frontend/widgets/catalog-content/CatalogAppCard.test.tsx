import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { CatalogAppCard } from '../../../../src/widgets/catalog-content/ui/CatalogAppCard/CatalogAppCard'
import type { AppInfo } from '../../../../src/entities/app'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../src/entities/category'

vi.mock('../../../../src/features/launch-app/model/useIsLaunching', () => ({
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

describe('CatalogAppCard', () => {
	it('renders non-draggable card actions', () => {
		render(
			<CatalogAppCard
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

		expect(screen.getByRole('button', { name: 'Manage Claude' })).toContainHTML(
			'lucide-ellipsis-vertical',
		)
		const favorite = screen.getByRole('button', {
			name: 'Add Claude to favorites',
		})
		expect(favorite).toHaveAttribute('aria-pressed', 'false')
		expect(favorite).not.toHaveClass('border')
	})

	it('keeps the selected favorite as a star without a colored button surface', () => {
		render(
			<CatalogAppCard
				app={app}
				isFavorite
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

		const favorite = screen.getByRole('button', {
			name: 'Remove Claude from favorites',
		})
		expect(favorite).toHaveAttribute('aria-pressed', 'true')
		expect(favorite).not.toHaveClass('bg-yellow-300/20')
	})
})
