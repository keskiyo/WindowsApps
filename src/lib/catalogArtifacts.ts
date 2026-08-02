import type { AppInfo } from '../types'

export const INSTALLERS_DOCS_CATEGORY = 'installers_docs'

export function isCatalogArtifact(app: AppInfo): boolean {
	return app.artifactKind === 'installer' || app.artifactKind === 'documentation'
}

export function isInstaller(app: AppInfo): boolean {
	return app.artifactKind === 'installer'
}
