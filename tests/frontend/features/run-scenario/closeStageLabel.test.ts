import { describe, expect, it } from 'vitest'
import { closeStageLabel } from '../../../../src/features/run-scenario/lib/closeStageLabel'
import { scenarioRunSummaryMessage } from '../../../../src/features/run-scenario'

describe('closeStageLabel', () => {
	// Closing takes five seconds of apparent nothing. The stage text is what tells the reader the
	// app is waiting on purpose rather than hung.
	it('names each stage of a close in the words of what is happening', () => {
		expect(
			closeStageLabel({ stage: 'asking', running: 2, secondsLeft: 0 }),
		).toBe('asking 2 apps to close')
		expect(
			closeStageLabel({ stage: 'asking', running: 1, secondsLeft: 0 }),
		).toBe('asking 1 app to close')
		expect(
			closeStageLabel({ stage: 'waiting', running: 0, secondsLeft: 3 }),
		).toBe('waiting 3 s before force-closing')
		expect(
			closeStageLabel({ stage: 'terminating', running: 0, secondsLeft: 0 }),
		).toBe('force-closing what stayed open')
	})
})

describe('scenarioRunSummaryMessage', () => {
	it('counts everything that happened, failures included', () => {
		expect(
			scenarioRunSummaryMessage({
				scenarioName: 'Gaming',
				launched: 3,
				launchFailed: 1,
				closed: 2,
				notRunning: 1,
				blocked: 1,
				closeFailed: 1,
				closeUnavailable: false,
			}),
		).toBe(
			'Gaming: 3 launched, 1 could not start, 2 closed, 1 already closed, 1 refused to close, 1 stayed open',
		)
	})

	it('says nothing when a scenario had nothing to do', () => {
		expect(
			scenarioRunSummaryMessage({
				scenarioName: 'Empty',
				launched: 0,
				launchFailed: 0,
				closed: 0,
				notRunning: 0,
				blocked: 0,
				closeFailed: 0,
				closeUnavailable: false,
			}),
		).toBeNull()
	})
})
