import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { InstallersDocsGrid } from '../../../../src/components/catalog/InstallersDocsGrid/InstallersDocsGrid'
import type { AppInfo } from '../../../../src/types'

vi.mock('../../../../src/components/apps/AppCard/AppCard', () => ({
	AppCard: ({ app }: { app: AppInfo }) => <article>{app.name}</article>,
}))

function app(id: string, artifactKind: 'installer' | 'documentation'): AppInfo {
	return {
		id,
		name: id,
		path: `C:\\${id}.exe`,
		iconBase64: null,
		artifactKind,
		category: 'installers_docs',
		launchKind: 'executable',
		sourceKind: 'portable',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
	}
}

const callbacks = {
	categoryOrder: ['installers_docs'],
	categories: [
		{
			id: 'installers_docs',
			label: 'Installers & Docs',
			builtIn: true,
		},
	],
	favoriteAppIds: [],
	onToggleFavorite: vi.fn(),
	onLaunch: vi.fn().mockResolvedValue(undefined),
	onMoveApp: vi.fn(),
	onInfo: vi.fn(),
	onUninstall: vi.fn(),
	onHide: vi.fn(),
	onRestore: vi.fn(),
	onDemoteAuxiliary: vi.fn(),
}

describe('InstallersDocsGrid', () => {
	it('partitions artifacts into counted groups', () => {
		render(
			<InstallersDocsGrid
				{...callbacks}
				apps={[
					app('Visual Studio Setup', 'installer'),
					app('Application Verifier Help', 'documentation'),
				]}
				hasQuery={false}
			/>,
		)

		expect(screen.getByRole('heading', { name: 'Installers 1' })).toBeVisible()
		expect(screen.getByRole('heading', { name: 'Docs 1' })).toBeVisible()
		expect(screen.getByText('Visual Studio Setup')).toBeVisible()
		expect(screen.getByText('Application Verifier Help')).toBeVisible()
	})

	it('omits an empty group and explains an empty filtered result', () => {
		const { rerender } = render(
			<InstallersDocsGrid
				{...callbacks}
				apps={[app('Very long installer product name', 'installer')]}
				hasQuery={false}
			/>,
		)
		expect(screen.queryByRole('heading', { name: /Docs/ })).not.toBeInTheDocument()

		rerender(
			<InstallersDocsGrid {...callbacks} apps={[]} hasQuery={true} />,
		)
		expect(screen.getByText('No matching installers or docs')).toBeVisible()
	})
})
