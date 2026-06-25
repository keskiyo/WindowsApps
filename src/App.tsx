import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { toast, Toaster } from 'sonner'
import { useStore } from 'zustand'
import type { StoreApi } from 'zustand/vanilla'
import { AppGrid } from './components/catalog/AppGrid'
import { AppInfoDialog } from './components/dialogs/AppInfoDialog'
import { UninstallDialog } from './components/dialogs/UninstallDialog'
import { AppDrawer } from './components/navigation/AppDrawer'
import { AppSidebar } from './components/navigation/AppSidebar'
import { SettingsPage } from './components/settings/SettingsPage'
import { Header } from './components/shared/Header'
import { ScanPrompt } from './components/shared/ScanPrompt'
import { TitleBar } from './components/shared/TitleBar'
import { useAppFeedback } from './hooks/useAppFeedback'
import { useCatalogNavigation } from './hooks/useCatalogNavigation'
import { useDesktopNavigation } from './hooks/useDesktopNavigation'
import { catalogChangeMessage } from './lib/catalogChanges'
import { tauriSystemClient } from './lib/system'
import {
	appStore,
	filterAppsByQuery,
	filterVisibleApps,
	selectCategorizedApps,
	type AppState,
} from './store/appStore'
import type { AppInfo, SystemClient, UninstallPreview } from './types'

interface AppProps {
	store?: StoreApi<AppState>
	systemClient?: SystemClient
}

export function App({
	store = appStore,
	systemClient = tauriSystemClient,
}: AppProps) {
	const state = useStore(store)
	// Dedup is O(N) but still recomputed only when the catalog actually changes; query
	// typing, scan progress, favorites and drawer toggles reuse the memoized result.
	const categorizedApps = useMemo(
		() => selectCategorizedApps(state),
		[state.apps, state.categoryOverrides],
	)
	const visibleApps = useMemo(
		() =>
			filterVisibleApps(
				categorizedApps,
				state.activeView,
				state.hiddenAppIds,
				state.favoriteAppIds,
			),
		[
			categorizedApps,
			state.activeView,
			state.hiddenAppIds,
			state.favoriteAppIds,
		],
	)
	const filteredApps = useMemo(
		() => filterAppsByQuery(visibleApps, state.query),
		[visibleApps, state.query],
	)
	const visibleHydrationIds = filteredApps
		.slice(0, 48)
		.map(app => app.id)
		.join('|')
	const [drawerOpen, setDrawerOpen] = useState(false)
	const [infoApp, setInfoApp] = useState<AppInfo | null>(null)
	const [uninstallApp, setUninstallApp] = useState<AppInfo | null>(null)
	const [uninstallPreview, setUninstallPreview] =
		useState<UninstallPreview | null>(null)
	const [uninstallPreviewLoading, setUninstallPreviewLoading] =
		useState(false)
	const [uninstallPreviewError, setUninstallPreviewError] = useState<
		string | null
	>(null)
	const [scanPromptDismissed, setScanPromptDismissed] = useState(false)
	const menuButtonRef = useRef<HTMLButtonElement>(null)
	const desktopNavigation = useDesktopNavigation()
	const closeDrawer = useCallback(() => setDrawerOpen(false), [])
	const closeInfo = useCallback(() => setInfoApp(null), [])
	const closeUninstall = useCallback(() => {
		setUninstallApp(null)
		setUninstallPreview(null)
		setUninstallPreviewError(null)
	}, [])
	const feedback = useAppFeedback({
		onLaunch: state.launch,
		onRefresh: state.refresh,
		onUninstall: state.uninstall,
	})
	const navigation = useCatalogNavigation({
		collapsedCategories: state.collapsedCategories,
		setActiveView: state.setActiveView,
		toggleCategory: state.toggleCategory,
		closeDrawer,
	})
	async function confirmUninstall() {
		if (!uninstallApp) return
		await feedback.uninstall(uninstallApp.id)
		setUninstallApp(null)
		try {
			await state.refresh()
		} catch {
			// The store exposes the refresh error through the existing toast effect.
		}
	}

	useEffect(() => {
		if (!uninstallApp) return
		let active = true
		setUninstallPreview(null)
		setUninstallPreviewError(null)
		setUninstallPreviewLoading(true)
		void state
			.getUninstallPreview(uninstallApp.id)
			.then(preview => {
				if (active) setUninstallPreview(preview)
			})
			.catch(error => {
				if (active)
					setUninstallPreviewError(
						error instanceof Error ? error.message : String(error),
					)
			})
			.finally(() => {
				if (active) setUninstallPreviewLoading(false)
			})
		return () => {
			active = false
		}
	}, [state.getUninstallPreview, uninstallApp])

	useEffect(() => {
		let dispose: (() => void) | undefined
		void state.initialize().then(value => {
			dispose = value
		})
		return () => {
			dispose?.()
		}
	}, [state.initialize])

	useEffect(() => {
		if (state.error) {
			toast.error(state.error)
		}
	}, [state.error])

	useEffect(() => {
		if (!state.catalogChange) return
		if (!state.isRefreshing) {
			const message = catalogChangeMessage(state.catalogChange)
			if (message) toast.success(message)
		}
		state.clearCatalogChange()
	}, [state.catalogChange, state.isRefreshing, state.clearCatalogChange])

	useEffect(() => {
		if (desktopNavigation) setDrawerOpen(false)
	}, [desktopNavigation])

	useEffect(() => {
		if (state.activeView === 'settings' || state.isLoading) return
		const ids = visibleHydrationIds.split('|').filter(Boolean)
		if (ids.length) void state.hydrateVisibleIcons(ids)
	}, [
		state.activeView,
		state.hydrateVisibleIcons,
		state.isLoading,
		visibleHydrationIds,
	])

	const visibleCategorizedApps = categorizedApps.filter(
		app => !state.hiddenAppIds.includes(app.id),
	)
	const navigationCounts = new Map<string, number>()
	for (const app of visibleCategorizedApps)
		navigationCounts.set(
			app.category,
			(navigationCounts.get(app.category) ?? 0) + 1,
		)
	const navigationProps = {
		categoryOrder: state.categoryOrder,
		categories: state.categories,
		counts: navigationCounts,
		activeView: state.activeView,
		favoriteCount: visibleCategorizedApps.filter(app =>
			state.favoriteAppIds.includes(app.id),
		).length,
		hiddenCount: state.hiddenAppIds.filter(id =>
			state.apps.some(app => app.id === id),
		).length,
		onSelectView: navigation.selectView,
		onSelectCategory: navigation.selectCategory,
		onReorderCategory: state.reorderCategory,
		onCreateCategory: state.createCategory,
	}

	return (
		<div className='app-shell theme-soft-surface flex h-screen flex-col overflow-hidden'>
			<TitleBar />
			<div className='flex min-h-0 flex-1 gap-2 px-2 pb-2'>
				{desktopNavigation && <AppSidebar {...navigationProps} />}
				<div
					id='catalog-scroll'
					className='app-panel flex min-h-0 flex-1 flex-col overflow-y-auto rounded-2xl'
				>
					<Header
					appCount={state.apps.length}
					query={state.query}
					isRefreshing={state.isRefreshing}
					scanProgress={state.scanProgress}
					onQueryChange={state.setQuery}
					onRefresh={feedback.refresh}
					onCancelScan={state.cancelScan}
					menuButtonRef={menuButtonRef}
					onOpenNavigation={() => setDrawerOpen(true)}
					onGoHome={navigation.goHome}
					showMenu={!desktopNavigation}
				/>
				<main className='mx-auto w-full max-w-375 px-5 pb-12 pt-7 sm:px-8'>
					{state.activeView === 'settings' ? (
						<SettingsPage
							client={systemClient}
							onForceFullScan={state.forceFullScan}
							onResetCatalogCache={state.resetCatalogCache}
						/>
					) : !state.isLoading &&
					  !state.hasCache &&
					  !state.apps.length &&
					  !scanPromptDismissed ? (
						<ScanPrompt
							isScanning={state.isRefreshing}
							onDismiss={() => setScanPromptDismissed(true)}
							onScan={feedback.refresh}
						/>
					) : (
						<AppGrid
							apps={filteredApps}
							isLoading={state.isLoading}
							hasQuery={state.query.trim().length > 0}
							activeView={state.activeView}
							categoryOrder={state.categoryOrder}
							categories={state.categories}
							collapsedCategories={state.collapsedCategories}
							favoriteAppIds={state.favoriteAppIds}
							onToggleCategory={state.toggleCategory}
							onToggleFavorite={state.toggleFavorite}
							onReorderCategory={state.reorderCategory}
							onMoveApp={state.moveApp}
							onLaunch={feedback.launch}
							onInfo={setInfoApp}
							onUninstall={setUninstallApp}
							onHide={state.hideApp}
							onRestore={state.restoreApp}
							onRenameCategory={state.renameCategory}
							onDeleteCategory={state.deleteCategory}
						/>
					)}
				</main>
				</div>
			</div>
			{drawerOpen && !desktopNavigation && (
				<AppDrawer
					apps={visibleCategorizedApps}
					categoryOrder={state.categoryOrder}
					categories={state.categories}
					activeView={state.activeView}
					favoriteCount={
						categorizedApps.filter(app =>
							state.favoriteAppIds.includes(app.id),
						).length
					}
					hiddenCount={navigationProps.hiddenCount}
					triggerRef={menuButtonRef}
					onSelectView={navigation.selectView}
					onSelectCategory={navigation.selectCategory}
					onReorderCategory={state.reorderCategory}
					onCreateCategory={state.createCategory}
					onClose={closeDrawer}
				/>
			)}
			{infoApp && (
				<AppInfoDialog
					app={infoApp}
					categories={state.categories}
					onClose={closeInfo}
				/>
			)}
			{uninstallApp && (
				<UninstallDialog
					appName={uninstallApp.name}
					preview={uninstallPreview}
					isPreviewLoading={uninstallPreviewLoading}
					previewError={uninstallPreviewError}
					onClose={closeUninstall}
					onConfirm={confirmUninstall}
				/>
			)}
			<Toaster
				className='app-toaster'
				theme='light'
				position='bottom-right'
				richColors
				closeButton
			/>
		</div>
	)
}
