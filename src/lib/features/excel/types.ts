/**
 * Shared Excel types — ports of the structs in `src-tauri/src/sf2/models.rs`.
 *
 * Owned here so the excel modules (`workbook`, `marks`, `roster`, `formulas`)
 * all agree on shapes without importing each other.
 */

/** One cell to write, plus the formula it should carry. */
export type Sf2CellMark = {
	sheetName: string;
	address: string;
	value: string;
	/** Formula without the leading `=`, or omitted for a literal. */
	formula?: string;
	/** Computed result Excel caches for the formula. */
	cachedValue?: number | string;
	/** `bold` applies SF2's heavier headers; other style names as needed. */
	style?: 'bold' | 'header' | 'plain';
};

export type Sf2WorkbookMetadata = {
	schoolId: string;
	schoolName: string;
	schoolYear: string;
	reportMonth: string;
	gradeLevel: string;
	section: string;
	adviserName: string;
	schoolHeadName: string;
	firstSchoolDay: number;
};

export type Sf2WorkbookAnalysis = {
	exists: boolean;
	sheets: Sf2SheetAnalysis[];
	/** True when the workbook has the sheets and metadata an SF2 month needs. */
	looksLikeSf2: boolean;
};

export type Sf2SheetAnalysis = {
	name: string;
	/** 1-based, matching Excel. */
	index: number;
	rowCount: number;
	columnCount: number;
	mergedRanges: string[];
	/** First non-empty cells, for the diagnose view. */
	sampleCells: Sf2SampleCell[];
	hasProtection: boolean;
};

export type Sf2SampleCell = {
	address: string;
	row: number;
	column: number;
	value: string;
	formula?: string;
};

/** One learner row as the workbook stores it. */
export type Sf2LearnerRow = {
	row: number;
	name: string;
	learnerId?: string;
	gender?: 'M' | 'F';
};

export type Sf2WorkbookPaths = {
	workbookPath: string;
	templatePath?: string;
};
