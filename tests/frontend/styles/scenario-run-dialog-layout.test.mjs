import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const dialog = readFileSync(
	'src/features/manage-scenarios/ui/ScenarioRunDialog/ScenarioRunDialog.tsx',
	'utf8',
)

describe('scenario run dialog layout', () => {
	it('fits inside the supported minimum window width', () => {
		expect(dialog).toContain('w-full max-w-xl')
		expect(dialog).not.toContain('min-w-xl')
	})
})
