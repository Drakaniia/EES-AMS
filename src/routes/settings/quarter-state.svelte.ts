/**
 * Quarter management state — current quarter selection and date ranges.
 * Consumed by QuarterDialog and GlobalConfigForm via settingsState.quarterState.
 */
export class QuarterState {
	defaultQuarter = $state('1st Quarter');
	q1Start = $state('');
	q1End = $state('');
	q2Start = $state('');
	q2End = $state('');
	q3Start = $state('');
	q3End = $state('');
	quarterDialogOpen = $state(false);
}

export const quarterState = new QuarterState();
