import {
	listStudents,
	saveStudent,
	createStudents,
	deleteStudent,
	listClasses,
	getSf2ExportReadiness,
	type Student,
	type StudentGender,
	type CreateStudentRequest,
	type Class,
	type Sf2ExportReadiness
} from '$lib/db-rust';
import { parseStudentNames, type EntryMode } from './student-state.svelte';

class StudentPageState {
	// ── State ────────────────────────────────────────────────────────────────
	students = $state<Student[]>([]);
	classes = $state<Class[]>([]);
	sf2Readiness = $state<Sf2ExportReadiness | null>(null);
	searchTerms = $state('');
	genderFilter = $state<'all' | 'male' | 'female'>('all');
	sortBy = $state<'name' | 'date'>('name');
	sortOrder = $state<'asc' | 'desc'>('asc');
	loading = $state(true);
	loadError = $state<string | null>(null);
	savingStudent = $state(false);

	dialogOpen = $state(false);
	attendanceModalOpen = $state(false);
	viewingStudent = $state<Student | null>(null);
	editing = $state<Student | null>(null);
	scanFor = $state<Student | null>(null);

	entryMode = $state<EntryMode>('single');
	formName = $state('');
	formGender = $state<StudentGender>('male');
	formCardSerial = $state('');
	formClassId = $state('');
	bulkMaleStudentNames = $state('');
	bulkFemaleStudentNames = $state('');

	deleteTarget = $state<Student | null>(null);

	cardSerial = $state('');

	toastMessage = $state<string | null>(null);
	toastTimer: ReturnType<typeof setTimeout> | null = null;

	// Pagination
	currentPage = $state(1);
	availableHeight = $state(0);

	// ── Derived ──────────────────────────────────────────────────────────────
	itemsPerPage = $derived.by(() => {
		if (this.availableHeight === 0) return 10;
		const rowHeight = 60;
		const headerHeight = 48;
		const verticalBuffer = 120;
		const calculated = Math.floor((this.availableHeight - headerHeight - verticalBuffer) / rowHeight);
		return Math.max(1, calculated);
	});

	filteredStudents = $derived.by(() => {
		let result = this.students;

		if (this.searchTerms.trim()) {
			const term = this.searchTerms.toLowerCase();
			result = result.filter((s) => s.name.toLowerCase().includes(term));
		}

		if (this.genderFilter !== 'all') {
			result = result.filter((s) => s.gender === this.genderFilter);
		}

		result = [...result].sort((a, b) => {
			let valA: string | number = '';
			let valB: string | number = '';

			if (this.sortBy === 'name') {
				valA = a.name;
				valB = b.name;
			} else if (this.sortBy === 'date') {
				valA = a.createdAt;
				valB = b.createdAt;
			}

			if (valA < valB) return this.sortOrder === 'asc' ? -1 : 1;
			if (valA > valB) return this.sortOrder === 'asc' ? 1 : -1;
			return 0;
		});

		return result;
	});

	totalPages = $derived(Math.ceil(this.filteredStudents.length / this.itemsPerPage));
	paginatedStudents = $derived.by(() => {
		const start = (this.currentPage - 1) * this.itemsPerPage;
		const end = start + this.itemsPerPage;
		return this.filteredStudents.slice(start, end);
	});
	bulkMaleNames = $derived.by(() => parseStudentNames(this.bulkMaleStudentNames));
	bulkFemaleNames = $derived.by(() => parseStudentNames(this.bulkFemaleStudentNames));
	bulkStudentCount = $derived(this.bulkMaleNames.length + this.bulkFemaleNames.length);
	maleStudentCount = $derived(
		this.filteredStudents.filter((student) => student.gender === 'male').length
	);
	femaleStudentCount = $derived(
		this.filteredStudents.filter((student) => student.gender === 'female').length
	);
	sf2Template = $derived(this.sf2Readiness?.template ?? null);
	assignedClass = $derived.by(() => {
		const sf2ClassId = this.sf2Template?.classId;
		return sf2ClassId ? (this.classes.find((classItem) => classItem.id === sf2ClassId) ?? null) : null;
	});
	canCreateStudents = $derived(Boolean(this.sf2Template && this.assignedClass));
	studentCreationBlockedMessage = $derived(
		this.sf2Template
			? 'The SF2 workbook class is unavailable. Recreate or import the SF2 workbook before adding students.'
			: 'Create an SF2 workbook before adding students.'
	);
	assignedClassLabel = $derived(
		this.assignedClass?.name ?? (this.sf2Template ? 'Class unavailable' : 'No SF2 workbook created')
	);

	// ── Constructor (effects) ────────────────────────────────────────────────
	constructor() {
		$effect.root(() => {
			$effect(() => {
				if (this.currentPage > this.totalPages && this.totalPages > 0) {
					this.currentPage = this.totalPages;
				}
			});

			$effect(() => {
				// Reset to first page when gender filter changes (results may be fewer)
				void this.genderFilter;
				this.currentPage = 1;
			});

			$effect(() => {
				if (this.dialogOpen && this.assignedClass && this.formClassId !== this.assignedClass.id) {
					this.formClassId = this.assignedClass.id;
				}
			});
		});
	}

	// ── Lifecycle ────────────────────────────────────────────────────────────
	async init() {
		await this.reload();
	}

	// ── Helpers ──────────────────────────────────────────────────────────────
	toast(msg: string) {
		this.toastMessage = msg;
		if (this.toastTimer) clearTimeout(this.toastTimer);
		this.toastTimer = setTimeout(() => (this.toastMessage = null), 3000);
	}

	setEntryMode(mode: EntryMode) {
		if (this.entryMode === mode) return;
		this.entryMode = mode;
	}

	handlePageChange(page: number) {
		this.currentPage = page;
	}

	toggleSort(field: typeof this.sortBy) {
		if (this.sortBy === field) {
			this.sortOrder = this.sortOrder === 'asc' ? 'desc' : 'asc';
		} else {
			this.sortBy = field;
			this.sortOrder = 'asc';
		}
	}

	async reload() {
		this.loading = true;
		this.loadError = null;
		try {
			const [s, c, readiness] = await Promise.all([
				listStudents(),
				listClasses(),
				getSf2ExportReadiness()
			]);
			this.students = s;
			this.classes = c;
			this.sf2Readiness = readiness;
			this.currentPage = 1;
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : 'Database error';
			this.loadError = msg;
			this.toast(`Failed to load students: ${msg}`);
		} finally {
			this.loading = false;
		}
	}

	// ── Dialog helpers ───────────────────────────────────────────────────────
	openAdd = () => {
		if (!this.canCreateStudents) {
			this.toast(this.studentCreationBlockedMessage);
			return;
		}

		this.editing = null;
		this.entryMode = 'single';
		this.formName = '';
		this.formGender = 'male';
		this.formCardSerial = '';
		this.formClassId = this.assignedClass?.id ?? '';
		this.bulkMaleStudentNames = '';
		this.bulkFemaleStudentNames = '';
		this.dialogOpen = true;
	};

	openEdit = (s: Student) => {
		this.editing = s;
		this.entryMode = 'single';
		this.formName = s.name;
		this.formGender = s.gender ?? 'male';
		this.formCardSerial = s.cardSerial ?? '';
		this.formClassId = this.assignedClass?.id ?? s.classId ?? '';
		this.bulkMaleStudentNames = '';
		this.bulkFemaleStudentNames = '';
		this.dialogOpen = true;
	};

	openAttendance = (s: Student) => {
		this.viewingStudent = s;
		this.attendanceModalOpen = true;
	};

	closeDialog = () => {
		this.dialogOpen = false;
		this.editing = null;
	};

	openScan = (s: Student) => {
		this.scanFor = s;
		this.cardSerial = s.cardSerial ?? '';
	};

	// ── CRUD operations ──────────────────────────────────────────────────────
	onSubmit = async (e: SubmitEvent) => {
		e.preventDefault();
		if (this.savingStudent) return;
		const name = this.formName.trim().toUpperCase();
		const serial = this.formCardSerial.trim().toLowerCase();
		const classId = this.formClassId || this.assignedClass?.id || '';

		if (!this.editing && !this.canCreateStudents) {
			this.toast(this.studentCreationBlockedMessage);
			return;
		}

		if (!this.editing && this.entryMode === 'bulk') {
			if (this.bulkStudentCount === 0) {
				this.toast('Paste or type at least one student name');
				return;
			}

			try {
				this.savingStudent = true;
				const studentRequests: CreateStudentRequest[] = [
					...this.bulkMaleNames.map((bulkName) => ({
						name: bulkName,
						gender: 'male' as const,
						classId: classId || undefined
					})),
					...this.bulkFemaleNames.map((bulkName) => ({
						name: bulkName,
						gender: 'female' as const,
						classId: classId || undefined
					}))
				];
				const createdStudents = await createStudents(studentRequests);
				this.students = [...createdStudents, ...this.students];
				this.toast(`${this.bulkStudentCount} ${this.bulkStudentCount === 1 ? 'student' : 'students'} added`);
				this.closeDialog();
			} catch (error) {
				const msg = error instanceof Error ? error.message : 'Failed to add students';
				this.toast(`Error: ${msg}`);
			} finally {
				this.savingStudent = false;
			}
			return;
		}

		if (!name) {
			this.toast('Please enter a student name');
			return;
		}
		try {
			this.savingStudent = true;
			const studentData: Student = this.editing
				? {
						...this.editing,
						name,
						gender: this.formGender,
						cardSerial: serial,
						classId
					}
				: {
						id: '',
						createdAt: new Date().toISOString(),
						name,
						gender: this.formGender,
						cardSerial: serial || undefined,
						classId: classId || undefined
					};

			const savedStudent = await saveStudent(studentData);
			this.students = this.editing
				? this.students.map((student) => (student.id === savedStudent.id ? savedStudent : student))
				: [savedStudent, ...this.students];
			this.toast(this.editing ? 'Student updated' : 'Student added');
			this.closeDialog();
		} catch (error) {
			console.error('Error saving student:', error);
			const msg = error instanceof Error ? error.message : 'Failed to save student';

			if (msg.includes('UNIQUE constraint failed') && msg.includes('card_serial')) {
				this.toast('Card serial already registered to another student.');
			} else {
				this.toast(`Error: ${msg}`);
			}
		} finally {
			this.savingStudent = false;
		}
	};

	confirmDelete = async (target = this.deleteTarget) => {
		if (!target) return;
		await deleteStudent(target.id);
		this.students = this.students.filter((student) => student.id !== target.id);
		this.toast('Deleted');
		this.deleteTarget = null;
	};

	onDelete = (event: MouseEvent, student: Student) => {
		if (event.shiftKey) {
			void this.confirmDelete(student);
			return;
		}
		this.deleteTarget = student;
	};

	onSaveCard = async () => {
		const serial = this.cardSerial.trim().toLowerCase();
		if (!this.scanFor || !serial) return;
		try {
			const savedStudent = await saveStudent({ ...this.scanFor, cardSerial: serial });
			this.students = this.students.map((student) =>
				student.id === savedStudent.id ? savedStudent : student
			);
			this.toast(`Card paired to ${this.scanFor.name}`);
			this.scanFor = null;
			this.cardSerial = '';
		} catch (error) {
			const msg = error instanceof Error ? error.message : 'Failed to pair card';
			this.toast(`Card pairing failed: ${msg}`);
		}
	};
}

export const studentPage = new StudentPageState();
