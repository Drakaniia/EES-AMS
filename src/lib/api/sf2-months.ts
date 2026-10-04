/**
 * `$lib/api/sf2-months.ts` → `$lib/features/sf2/month/month`.
 *
 * A pure re-export: the seven month commands were ported against this call surface
 * name for name, and the Reports page's month switch is not allowed to change
 * (spec D9, §7.1).
 */

import { syncMonthRosterForClass as syncRoster } from '$lib/features/sf2/month/roster-sync';

export {
	createSf2MonthFile,
	getSf2LaunchMonth,
	getSf2MonthPreview,
	getSf2SchoolCalendarSettings,
	listSf2MonthWorkbooks,
	runSf2WorkbookSplit,
	setSf2SchoolStartDate
} from '$lib/features/sf2/month/month';

/**
 * Put a class's roster onto its SF2 month worksheets, and return how many learners
 * are mapped.
 *
 * Every Students-page save calls this, so a learner is on the SF2 grid as soon as
 * they are on file - there is no button to press. A class with no SF2 month on
 * record maps nothing and is not an error.
 */
export async function refreshSf2MonthRoster(classId: string): Promise<number> {
	return await syncRoster(classId);
}

export type {
	Sf2LaunchMonth,
	Sf2MonthGridPreview,
	Sf2MonthPreview,
	Sf2MonthTemplate,
	Sf2SchoolCalendarSettings,
	Sf2SplitOutcome
} from '$lib/types';
