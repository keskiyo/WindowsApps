export interface StaleCopyBannerProps {
	installedVersion: string
	installLocation: string
	onOpenInstalled(): Promise<void>
	onDismiss(): void
}
