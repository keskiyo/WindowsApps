import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { MorePage, type MorePageProps } from '../../../../src/pages/more'
import type { AppInfo } from '../../../../src/entities/app'

function app(value: Partial<AppInfo> & Pick<AppInfo, 'id' | 'name'>): AppInfo {
	return {
		path: `C:\\Apps\\${value.id}.exe`,
		category: 'other',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
		...value,
	}
}

type MorePreview = MorePageProps['preview']

function previewEntry(
	appInfo: AppInfo,
	firstSeenAt: number | null = null,
): MorePreview['auxiliary'][number] {
	return { app: appInfo, firstSeenAt }
}

const emptyPreview: MorePreview = {
	auxiliary: [],
	hidden: [],
	installersDocs: [],
	scenarios: [],
}

function runControl(
	value: Partial<MorePageProps['scenarioRun']> = {},
): MorePageProps['scenarioRun'] {
	return {
		scenarios: [],
		apps: [],
		runningId: null,
		isScenarioRunning: false,
		onRun: vi.fn(),
		...value,
	}
}

function renderPage(
	onSelectView = vi.fn(),
	preview: MorePreview = emptyPreview,
	scenarioRun: MorePageProps['scenarioRun'] = runControl(),
) {
	render(
		<MorePage
			auxiliaryCount={78}
			hiddenCount={4}
			installersDocsCount={12}
			scenarioCount={2}
			preview={preview}
			scenarioRun={scenarioRun}
			onSelectView={onSelectView}
		/>,
	)
	return onSelectView
}

describe('MorePage', () => {
	it('describes More as secondary catalog views', () => {
		renderPage()

		expect(screen.getByText('Secondary catalog views.')).toBeInTheDocument()
	})

	it('shows the More symbol in the page header', () => {
		renderPage()

		const page = screen.getByRole('region', { name: 'More' })
		const header = page.querySelector(':scope > header')
		expect(header).not.toBeNull()
		expect(header?.querySelector('svg.lucide-wand-sparkles')).not.toBeNull()
	})

	it('opens each catalog view it collects', async () => {
		const onSelectView = renderPage()

		await userEvent.click(
			screen.getByRole('button', { name: 'Auxiliary tools 78' }),
		)
		expect(onSelectView).toHaveBeenCalledWith('auxiliary')

		await userEvent.click(screen.getByRole('button', { name: 'Hidden 4' }))
		expect(onSelectView).toHaveBeenCalledWith('hidden')

		await userEvent.click(
			screen.getByRole('button', { name: 'Installers & Docs 12' }),
		)
		expect(onSelectView).toHaveBeenCalledWith('installers_docs')
	})

	it('shows the count of each destination', () => {
		renderPage()

		expect(
			screen.getByRole('button', { name: 'Auxiliary tools 78' }),
		).toHaveTextContent('78')
		expect(
			screen.getByRole('button', { name: 'Installers & Docs 12' }),
		).toHaveTextContent('12')
	})

	it('previews the newest entries of each area', () => {
		renderPage(vi.fn(), {
			auxiliary: [
				previewEntry(
					app({ id: 'upd', name: 'Adobe Updater', publisher: 'Adobe' }),
				),
			],
			hidden: [previewEntry(app({ id: 'old', name: 'Old launcher' }))],
			scenarios: [],
			installersDocs: [
				previewEntry(
					app({ id: 'setup', name: 'setup.msi', artifactKind: 'installer' }),
				),
				previewEntry(
					app({
						id: 'manual',
						name: 'manual.pdf',
						artifactKind: 'documentation',
					}),
				),
			],
		})

		const artifacts = screen.getByRole('list', {
			name: 'Recently added to Installers & Docs',
		})
		expect(within(artifacts).getAllByRole('listitem')).toHaveLength(2)
		// The subtitle says only what the record actually carries.
		expect(artifacts).toHaveTextContent('Installer')
		expect(artifacts).toHaveTextContent('Documentation')
		expect(
			screen.getByRole('list', { name: 'Recently added to Auxiliary tools' }),
		).toHaveTextContent('Adobe')
		expect(
			screen.getByRole('list', { name: 'Recently added to Hidden' }),
		).toHaveTextContent('Old launcher')
	})

	it('shows the catalog first-seen date only when it is available', () => {
		const seenAt = new Date(2026, 7, 4, 12).getTime()
		renderPage(vi.fn(), {
			auxiliary: [
				previewEntry(app({ id: 'upd', name: 'Adobe Updater' }), seenAt),
			],
			hidden: [previewEntry(app({ id: 'old', name: 'Old launcher' }))],
			installersDocs: [],
			scenarios: [],
		})

		expect(
			screen.getByLabelText('First seen in catalog 04.08.2026'),
		).toBeInTheDocument()
		expect(
			within(
				screen.getByRole('list', { name: 'Recently added to Hidden' }),
			).queryByLabelText(/^First seen in catalog /),
		).not.toBeInTheDocument()
	})

	it('omits malformed catalog first-seen dates', () => {
		renderPage(vi.fn(), {
			auxiliary: [previewEntry(app({ id: 'zero', name: 'Zero' }), 0)],
			hidden: [
				previewEntry(
					app({ id: 'infinite', name: 'Infinite' }),
					Number.POSITIVE_INFINITY,
				),
			],
			scenarios: [],
			installersDocs: [
				previewEntry(
					app({ id: 'invalid', name: 'Invalid' }),
					Number.MAX_SAFE_INTEGER,
				),
			],
		})

		for (const label of [
			'Auxiliary tools',
			'Hidden',
			'Installers & Docs',
		])
			expect(
				within(
					screen.getByRole('list', { name: `Recently added to ${label}` }),
				).queryByLabelText(/^First seen in catalog /),
			).not.toBeInTheDocument()
	})

	it('opens the scenarios page and previews the scenarios by name', async () => {
		const onSelectView = renderPage(vi.fn(), {
			...emptyPreview,
			scenarios: [
				{
					id: 'gaming',
					name: 'Gaming',
					launchCount: 3,
					closeCount: 2,
					createdAt: new Date(2026, 7, 4, 12).getTime(),
				},
			],
		})

		const preview = screen.getByRole('list', {
			name: 'Recently added to Scenarios',
		})
		expect(preview).toHaveTextContent('Gaming')
		// What a scenario has instead of a publisher: the size of each of its two lists.
		expect(preview).toHaveTextContent('3 launch · 2 close')

		await userEvent.click(screen.getByRole('button', { name: 'Scenarios 2' }))
		expect(onSelectView).toHaveBeenCalledWith('scenarios')
	})

	// The card is the second place a scenario can be started from, so the row runs it without
	// making the user open the scenarios page first.
	it('runs a scenario from its preview row', async () => {
		const onRun = vi.fn()
		renderPage(
			vi.fn(),
			{
				...emptyPreview,
				scenarios: [
					{
						id: 'gaming',
						name: 'Gaming',
						launchCount: 3,
						closeCount: 2,
						createdAt: null,
					},
				],
			},
			runControl({ onRun }),
		)

		await userEvent.click(screen.getByRole('button', { name: 'Run Gaming' }))

		expect(onRun).toHaveBeenCalledWith('gaming')
	})

	// A second click while apps are still starting would launch everything twice.
	it('blocks every preview scenario until the active run finishes', () => {
		renderPage(
			vi.fn(),
			{
				...emptyPreview,
				scenarios: [
					{
						id: 'gaming',
						name: 'Gaming',
						launchCount: 1,
						closeCount: 0,
						createdAt: null,
					},
					{
						id: 'work',
						name: 'Work',
						launchCount: 1,
						closeCount: 0,
						createdAt: null,
					},
				],
			},
			runControl({ runningId: 'gaming', isScenarioRunning: true }),
		)

		const run = screen.getByRole('button', { name: 'Run Gaming' })
		expect(run).toBeDisabled()
		expect(run).toHaveTextContent('Running…')
		expect(
			screen.getByRole('button', {
				name: 'Run Work unavailable while another scenario is running',
			}),
		).toBeDisabled()
	})

	it('does not render legacy scenario history', () => {
		render(
			<MorePage
				{...({
					auxiliaryCount: 0,
					hiddenCount: 0,
					installersDocsCount: 0,
					scenarioCount: 0,
					preview: emptyPreview,
					scenarioRun: runControl(),
					onSelectView: vi.fn(),
					scenarioHistory: [
						{
							id: 'old-run',
							scenarioId: 'gaming',
							scenarioName: 'Gaming',
							startedAt: 1,
							finishedAt: 2,
							launched: 1,
							closed: 0,
							notRunning: 0,
							unavailable: 0,
							blocked: 0,
							failed: 0,
							cancelled: false,
						},
					],
				} as unknown as MorePageProps)}
			/>,
		)

		expect(
			screen.queryByRole('heading', { name: 'Scenario history' }),
		).not.toBeInTheDocument()
	})

	// Only this card: the preview cuts the list off, and the rest of it is something to run.
	it('opens every scenario over the page from the scenarios card', async () => {
		renderPage(
			vi.fn(),
			{
				...emptyPreview,
				scenarios: [
					{
						id: 'gaming',
						name: 'Gaming',
						launchCount: 1,
						closeCount: 0,
						createdAt: null,
					},
				],
			},
			runControl({
				scenarios: [
					{
						id: 'work',
						name: 'Work',
						launchIdentities: [],
						closeIdentities: [],
						createdAt: null,
					},
				],
			}),
		)

		expect(screen.getAllByRole('button', { name: /^View all/ })).toHaveLength(1)
		await userEvent.click(screen.getByRole('button', { name: /^View all/ }))

		const dialog = screen.getByRole('dialog', { name: 'All scenarios' })
		// The dialog shows the whole list, not the three the card previewed.
		expect(dialog).toHaveTextContent('Work')
		expect(dialog.parentElement?.parentElement).toBe(document.body)
	})

	it('omits the preview list for an empty area', () => {
		renderPage()

		expect(screen.queryAllByRole('list')).toEqual([])
	})

	it('stays usable when a destination is empty', async () => {
		const onSelectView = vi.fn()
		render(
			<MorePage
				auxiliaryCount={0}
				hiddenCount={0}
				installersDocsCount={0}
				scenarioCount={0}
				preview={emptyPreview}
				scenarioRun={runControl()}
				onSelectView={onSelectView}
			/>,
		)

		await userEvent.click(screen.getByRole('button', { name: 'Hidden 0' }))
		expect(onSelectView).toHaveBeenCalledWith('hidden')
	})
})
