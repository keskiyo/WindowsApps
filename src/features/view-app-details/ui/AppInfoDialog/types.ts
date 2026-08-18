import type { LucideIcon } from 'lucide-react'
import type { ReactNode, RefObject } from 'react'
import type { AppDetails, AppInfo, AppsClient } from '../../../../entities/app'
import type { CategoryDefinition } from '../../../../entities/category'

export interface AppInfoDialogProps {
	app: AppInfo
	categories: CategoryDefinition[]
	appsClient: Pick<AppsClient, 'getAppDetails' | 'openAppFolder'>
	onClose(): void
}

export interface AdditionalInformationProps {
	details: AppDetails | null
	isLoading: boolean
	hasKnownPackageInstallLocation: boolean
	hasPackageLaunchTarget: boolean
}

export interface AppIdentityHeaderProps {
	app: AppInfo
	categories: CategoryDefinition[]
	details: AppDetails | null
	closeRef: RefObject<HTMLButtonElement>
	onClose(): void
}

export interface AppInformationCardsProps {
	app: AppInfo
	categories: CategoryDefinition[]
	details: AppDetails | null
	isLoading: boolean
}

export interface DetailRowsProps {
	rows: [string, string][]
}

export interface DialogActionsProps {
	canOpenFolder: boolean
	isOpeningFolder: boolean
	message: string | null
	onCopyPath(): Promise<void>
	onCopyReport(): Promise<void>
	onOpenFolder(): Promise<void>
}

export interface InfoCardProps {
	icon: LucideIcon
	title: string
	children: ReactNode
}
