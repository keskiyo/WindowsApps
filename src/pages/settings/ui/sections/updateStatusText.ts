import type { UpdateCheckStatus } from '../../../../features/update-app'

export function updateStatusText(status: UpdateCheckStatus): string {
	switch (status) {
		case 'checking':
			return 'Checking for updates...'
		case 'current':
			return 'You are running the latest version.'
		case 'available':
			return 'Update available.'
		case 'error':
			return 'Could not check for updates.'
		default:
			return 'Check for updates or open the project repository.'
	}
}
