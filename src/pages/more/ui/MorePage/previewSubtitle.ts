import type { AppInfo } from '../../../../entities/app'

export function previewSubtitleOf(app: AppInfo): string | null {
	if (app.artifactKind === 'installer') return 'Installer'
	if (app.artifactKind === 'documentation') return 'Documentation'
	return app.publisher
}
