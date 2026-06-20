import { useCallback, useEffect, useRef, useState } from 'react'
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
import { useAppFeedback } from './hooks/useAppFeedback'
import { useCatalogNavigation } from './hooks/useCatalogNavigation'
import { useDesktopNavigation } from './hooks/useDesktopNavigation'
import { tauriSystemClient } from './lib/system'
import {
	appStore,
	selectCategorizedApps,
	selectFilteredApps,
	type AppState,
} from './store/appStore'
import type { AppInfo, SystemClient } from './types'

interface AppProps {
	store?: StoreApi<AppState>
	systemClient?: SystemClient
}

export function App({ store = appStore, systemClient = tauriSystemClient }: AppProps) {
	const state = useStore(store)
	const filteredApps = selectFilteredApps(state)
	const categorizedApps = selectCategorizedApps(state)
	const [drawerOpen, setDrawerOpen] = useState(false)
	const [infoApp, setInfoApp] = useState<AppInfo | null>(null)
	const [uninstallApp, setUninstallApp] = useState<AppInfo | null>(null)
	const [scanPromptDismissed, setScanPromptDismissed] = useState(false)
	const menuButtonRef = useRef<HTMLButtonElement>(null)
	const desktopNavigation = useDesktopNavigation()
	const closeDrawer = useCallback(() => setDrawerOpen(false), [])
	const closeInfo = useCallback(() => setInfoApp(null), [])
	const closeUninstall = useCallback(() => setUninstallApp(null), [])
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
		void state.load()
		let unlisten: (() => void) | undefined
		let unlistenProgress: (() => void) | undefined
		void state.subscribe().then(dispose => {
			unlisten = dispose
		})
		void state.subscribeScanProgress().then(dispose => {
			unlistenProgress = dispose
		})
		return () => {
			unlisten?.()
			unlistenProgress?.()
		}
	}, [state.load, state.subscribe, state.subscribeScanProgress])

	useEffect(() => {
		if (state.error) {
			toast.error(state.error)
		}
	}, [state.error])

	useEffect(() => {
		if (desktopNavigation) setDrawerOpen(false)
	}, [desktopNavigation])

	const visibleCategorizedApps = categorizedApps.filter(
		app => !state.hiddenAppIds.includes(app.id),
	)
	const navigationCounts = new Map<string, number>()
	for (const app of visibleCategorizedApps)
		navigationCounts.set(app.category, (navigationCounts.get(app.category) ?? 0) + 1)
	const navigationProps = {
		categoryOrder: state.categoryOrder,
		categories: state.categories,
		counts: navigationCounts,
		activeView: state.activeView,
		favoriteCount: visibleCategorizedApps.filter(app => state.favoriteAppIds.includes(app.id)).length,
		hiddenCount: state.hiddenAppIds.filter(id => state.apps.some(app => app.id === id)).length,
		onSelectView: navigation.selectView,
		onSelectCategory: navigation.selectCategory,
		onReorderCategory: state.reorderCategory,
		onCreateCategory: state.createCategory,
	}

	return (
		<div className='app-shell min-h-screen text-slate-100'>
			{desktopNavigation && <AppSidebar {...navigationProps} />}
			<div className={desktopNavigation ? 'ml-70' : ''}>
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
			<main className='mx-auto w-full max-w-375 px-5 pb-12 pt-7 sm:px-8'>
				{state.activeView === 'settings' ? (
					<SettingsPage client={systemClient} />
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
					onClose={closeUninstall}
					onConfirm={confirmUninstall}
				/>
			)}
			<Toaster
				className='app-toaster'
				theme='dark'
				position='bottom-right'
				richColors
				closeButton
			/>
		</div>
	)
}
