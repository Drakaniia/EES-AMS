import { FC, useEffect, useState, useRef } from 'react'
import type { Class, Student } from '../lib/tauri'

interface ImportResult {
    success_count: number
    error_count: number
    errors: string[]
    imported_students: Student[]
}

const Students: FC = () => {
    const [classes, setClasses] = useState<Class[]>([])
    const [selectedClassId, setSelectedClassId] = useState<number | null>(null)
    const [students, setStudents] = useState<Student[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [showModal, setShowModal] = useState(false)
    const [showImportModal, setShowImportModal] = useState(false)
    const [formData, setFormData] = useState({ student_id: '', first_name: '', last_name: '' })
    const [isSubmitting, setIsSubmitting] = useState(false)
    const [isImporting, setIsImporting] = useState(false)
    const [importResult, setImportResult] = useState<ImportResult | null>(null)
    const [searchQuery, setSearchQuery] = useState('')
    const fileInputRef = useRef<HTMLInputElement>(null)

    useEffect(() => {
        loadClasses()
    }, [])

    useEffect(() => {
        if (selectedClassId) {
            loadStudents()
        } else {
            loadAllStudents()
        }
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [selectedClassId])

    const loadClasses = async () => {
        try {
            const response = await window.electronAPI.class.getAll()
            if (response.success && response.data) {
                setClasses(response.data)
            }
        } catch (error) {
            console.error('Failed to load classes:', error)
        }
    }

    const loadStudents = async () => {
        if (!selectedClassId) return
        setIsLoading(true)
        try {
            const response = await window.electronAPI.student.getByClass(selectedClassId)
            if (response.success && response.data) {
                setStudents(response.data)
            }
        } catch (error) {
            console.error('Failed to load students:', error)
        } finally {
            setIsLoading(false)
        }
    }

    const loadAllStudents = async () => {
        setIsLoading(true)
        try {
            const response = await window.electronAPI.student.getAll()
            if (response.success && response.data) {
                setStudents(response.data)
            }
        } catch (error) {
            console.error('Failed to load students:', error)
        } finally {
            setIsLoading(false)
        }
    }

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        if (!formData.student_id.trim() || !formData.first_name.trim() || !formData.last_name.trim()) return

        setIsSubmitting(true)
        try {
            const response = await window.electronAPI.student.create({
                student_id: formData.student_id.trim(),
                first_name: formData.first_name.trim(),
                last_name: formData.last_name.trim(),
                class_id: selectedClassId || undefined
            })

            if (response.success) {
                setFormData({ student_id: '', first_name: '', last_name: '' })
                setShowModal(false)
                if (selectedClassId) {
                    loadStudents()
                } else {
                    loadAllStudents()
                }
            }
        } catch (error) {
            console.error('Failed to create student:', error)
        } finally {
            setIsSubmitting(false)
        }
    }

const handleDelete = async (id: number) => {
        if (!confirm('Are you sure you want to delete this student?')) return

        try {
            const response = await window.electronAPI.student.delete(id)
            if (response.success) {
                if (selectedClassId) {
                    loadStudents()
                } else {
                    loadAllStudents()
                }
            }
        } catch (error) {
            console.error('Failed to delete student:', error)
        }
    }

    const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0]
        if (!file) return

        // Check file extension
        if (!file.name.endsWith('.xls') && !file.name.endsWith('.xlsx')) {
            alert('Please select an Excel file (.xls or .xlsx)')
            return
        }

        setIsImporting(true)
        setImportResult(null)

        try {
            // Read file as ArrayBuffer and send to Tauri
            const arrayBuffer = await file.arrayBuffer()
            const uint8Array = new Uint8Array(arrayBuffer)
            
            // Save file temporarily and import
            const tempPath = `/tmp/${file.name}`
            await window.electronAPI.fs.writeFile(tempPath, uint8Array)
            
            const result = await window.electronAPI.student.importFromExcel(
                tempPath,
                selectedClassId || undefined
            )

            setImportResult(result)

            if (result.success_count > 0) {
                // Reload students
                if (selectedClassId) {
                    loadStudents()
                } else {
                    loadAllStudents()
                }
            }

            // Clean up temp file
            await window.electronAPI.fs.removeFile(tempPath)
        } catch (error) {
            console.error('Import failed:', error)
            setImportResult({
                success_count: 0,
                error_count: 1,
                errors: ['Import failed: ' + (error as Error).message],
                imported_students: []
            })
        } finally {
            setIsImporting(false)
        }
    }

    const triggerFileSelect = () => {
        fileInputRef.current?.click()
    }

    const filteredStudents = students.filter(s =>
        s.first_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.last_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.student_id.toLowerCase().includes(searchQuery.toLowerCase())
    )

    const getClassName = (classId?: number) => {
        if (!classId) return 'Unassigned'
        const cls = classes.find(c => c.id === classId)
        return cls ? `${cls.name}${cls.section ? ` - ${cls.section}` : ''}` : 'Unknown'
    }

    return (
        <div className="space-y-6">
{/* Header */}
            <div className="flex items-center justify-between">
                <div>
                    <h1 className="text-2xl font-bold text-white mb-1">Students</h1>
                    <p className="text-gray-400">Manage students across all classes</p>
                </div>
                <div className="flex gap-3">
                    <button onClick={() => setShowImportModal(true)} className="btn btn-secondary">
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                        </svg>
                        Import from Excel
                    </button>
                    <button onClick={() => setShowModal(true)} className="btn btn-primary">
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                        </svg>
                        Add Student
                    </button>
                </div>
            </div>

            {/* Filters */}
            <div className="glass rounded-2xl p-6">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div>
                        <label className="block text-sm text-gray-400 mb-2">Filter by Class</label>
                        <select
                            value={selectedClassId || ''}
                            onChange={(e) => setSelectedClassId(e.target.value ? Number(e.target.value) : null)}
                            className="input"
                        >
                            <option value="">All Classes</option>
                            {classes.map((cls) => (
                                <option key={cls.id} value={cls.id}>
                                    {cls.name} {cls.section && `- ${cls.section}`}
                                </option>
                            ))}
                        </select>
                    </div>
                    <div>
                        <label className="block text-sm text-gray-400 mb-2">Search</label>
                        <input
                            type="text"
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            placeholder="Search by name or ID..."
                            className="input"
                        />
                    </div>
                </div>
            </div>

            {/* Students Table */}
            {isLoading ? (
                <div className="flex items-center justify-center h-48">
                    <div className="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
                </div>
            ) : filteredStudents.length > 0 ? (
                <div className="glass rounded-2xl overflow-hidden">
                    <table className="w-full">
                        <thead className="bg-white/5">
                            <tr>
                                <th className="text-left text-sm text-gray-400 font-medium p-4">Student</th>
                                <th className="text-left text-sm text-gray-400 font-medium p-4">Student ID</th>
                                <th className="text-left text-sm text-gray-400 font-medium p-4">Class</th>
                                <th className="text-right text-sm text-gray-400 font-medium p-4">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-white/5">
                            {filteredStudents.map((student) => (
                                <tr key={student.id} className="hover:bg-white/5 transition-colors">
                                    <td className="p-4">
                                        <div className="flex items-center gap-3">
                                            <div className="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-500 flex items-center justify-center text-white font-medium">
                                                {student.first_name[0]}{student.last_name[0]}
                                            </div>
                                            <div>
                                                <p className="text-white font-medium">{student.first_name} {student.last_name}</p>
                                            </div>
                                        </div>
                                    </td>
                                    <td className="p-4 text-gray-400">{student.student_id}</td>
                                    <td className="p-4">
                                        <span className="badge badge-success">{getClassName(student.class_id)}</span>
                                    </td>
                                    <td className="p-4 text-right">
                                        <button
                                            onClick={() => handleDelete(student.id!)}
                                            className="p-2 hover:bg-red-500/20 rounded-lg transition-colors"
                                        >
                                            <svg className="w-5 h-5 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                            </svg>
                                        </button>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            ) : (
                <div className="glass rounded-2xl p-12 text-center">
                    <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-gradient-to-br from-blue-500/20 to-purple-500/20 flex items-center justify-center">
                        <svg className="w-10 h-10 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197" />
                        </svg>
                    </div>
                    <h3 className="text-xl font-semibold text-white mb-2">No Students Found</h3>
                    <p className="text-gray-400 mb-6">
                        {searchQuery ? 'No students match your search' : 'Add students to get started'}
                    </p>
                    <button onClick={() => setShowModal(true)} className="btn btn-primary">
                        Add Your First Student
                    </button>
                </div>
            )}

            {/* Modal */}
            {showModal && (
                <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
                    <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={() => setShowModal(false)}></div>
                    <div className="glass rounded-2xl p-6 w-full max-w-md relative animate-fade-in">
                        <h2 className="text-xl font-semibold text-white mb-6">Add New Student</h2>

                        <form onSubmit={handleSubmit} className="space-y-4">
                            <div>
                                <label className="block text-sm text-gray-400 mb-2">Student ID *</label>
                                <input
                                    type="text"
                                    value={formData.student_id}
                                    onChange={(e) => setFormData({ ...formData, student_id: e.target.value })}
                                    placeholder="e.g., STU-2025-001"
                                    className="input"
                                    required
                                />
                            </div>

                            <div className="grid grid-cols-2 gap-4">
                                <div>
                                    <label className="block text-sm text-gray-400 mb-2">First Name *</label>
                                    <input
                                        type="text"
                                        value={formData.first_name}
                                        onChange={(e) => setFormData({ ...formData, first_name: e.target.value })}
                                        placeholder="First name"
                                        className="input"
                                        required
                                    />
                                </div>
                                <div>
                                    <label className="block text-sm text-gray-400 mb-2">Last Name *</label>
                                    <input
                                        type="text"
                                        value={formData.last_name}
                                        onChange={(e) => setFormData({ ...formData, last_name: e.target.value })}
                                        placeholder="Last name"
                                        className="input"
                                        required
                                    />
                                </div>
                            </div>

                            <div>
                                <label className="block text-sm text-gray-400 mb-2">Assign to Class</label>
                                <select
                                    value={selectedClassId || ''}
                                    onChange={(e) => setSelectedClassId(e.target.value ? Number(e.target.value) : null)}
                                    className="input"
                                >
                                    <option value="">No class assigned</option>
                                    {classes.map((cls) => (
                                        <option key={cls.id} value={cls.id}>
                                            {cls.name} {cls.section && `- ${cls.section}`}
                                        </option>
                                    ))}
                                </select>
                            </div>

                            <div className="flex gap-3 pt-4">
                                <button
                                    type="button"
                                    onClick={() => setShowModal(false)}
                                    className="btn btn-secondary flex-1"
                                >
                                    Cancel
                                </button>
                                <button
                                    type="submit"
                                    disabled={isSubmitting}
                                    className="btn btn-primary flex-1"
                                >
                                    {isSubmitting ? 'Adding...' : 'Add Student'}
                                </button>
                            </div>
                        </form>
                    </div>
</div>
            )}

            {/* Import Modal */}
            {showImportModal && (
                <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
                    <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={() => setShowImportModal(false)}></div>
                    <div className="glass rounded-2xl p-6 w-full max-w-lg relative animate-fade-in">
                        <h2 className="text-xl font-semibold text-white mb-6">Import Students from Excel</h2>
                        
                        <div className="space-y-4">
                            <div className="glass rounded-lg p-4">
                                <h3 className="text-sm font-medium text-white mb-2">Supported Formats:</h3>
                                <ul className="text-sm text-gray-400 space-y-1">
                                    <li>• SF1 Excel files (.xls, .xlsx)</li>
                                    <li>• Columns: LRN, Last Name, First Name, Middle Name, etc.</li>
                                </ul>
                            </div>

                            <input
                                ref={fileInputRef}
                                type="file"
                                accept=".xls,.xlsx"
                                onChange={handleFileSelect}
                                className="hidden"
                            />

                            <button
                                onClick={triggerFileSelect}
                                disabled={isImporting}
                                className="w-full btn btn-primary"
                            >
                                {isImporting ? (
                                    <div className="flex items-center justify-center gap-2">
                                        <div className="w-5 h-5 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
                                        Importing...
                                    </div>
                                ) : (
                                    <div className="flex items-center justify-center gap-2">
                                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                                        </svg>
                                        Select Excel File
                                    </div>
                                )}
                            </button>

                            {/* Import Results */}
                            {importResult && (
                                <div className="space-y-3">
                                    <div className="glass rounded-lg p-4">
                                        <div className="flex items-center gap-3 mb-3">
                                            {importResult.success_count > 0 && (
                                                <div className="w-10 h-10 rounded-full bg-green-500/20 flex items-center justify-center">
                                                    <svg className="w-6 h-6 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                                                    </svg>
                                                </div>
                                            )}
                                            {importResult.error_count > 0 && (
                                                <div className="w-10 h-10 rounded-full bg-red-500/20 flex items-center justify-center">
                                                    <svg className="w-6 h-6 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                                                    </svg>
                                                </div>
                                            )}
                                            <div>
                                                <p className="text-white font-medium">
                                                    Import Complete
                                                </p>
                                                <p className="text-sm text-gray-400">
                                                    {importResult.success_count} students imported, {importResult.error_count} errors
                                                </p>
                                            </div>
                                        </div>

                                        {importResult.errors.length > 0 && (
                                            <div className="space-y-2">
                                                <p className="text-sm font-medium text-red-400">Errors:</p>
                                                <div className="max-h-32 overflow-y-auto space-y-1">
                                                    {importResult.errors.map((error, idx) => (
                                                        <p key={idx} className="text-xs text-gray-400">{error}</p>
                                                    ))}
                                                </div>
                                            </div>
                                        )}
                                    </div>
                                </div>
                            )}

                            <div className="flex gap-3 pt-4">
                                <button
                                    type="button"
                                    onClick={() => {
                                        setShowImportModal(false)
                                        setImportResult(null)
                                        if (fileInputRef.current) {
                                            fileInputRef.current.value = ''
                                        }
                                    }}
                                    className="btn btn-secondary flex-1"
                                >
                                    Close
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    )
}

export default Students
