import { FC, useEffect, useState } from 'react'
import type { Class, Student, AttendanceRecord } from '../lib/tauri'

type AttendanceStatus = 'present' | 'absent' | 'late' | 'excused'

interface StudentWithAttendance extends Student {
    attendanceStatus?: AttendanceStatus
    notes?: string
}

const Attendance: FC = () => {
    const [classes, setClasses] = useState<Class[]>([])
    const [selectedClassId, setSelectedClassId] = useState<number | null>(null)
    const [selectedDate, setSelectedDate] = useState(new Date().toISOString().split('T')[0])
    const [students, setStudents] = useState<StudentWithAttendance[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [isSaving, setIsSaving] = useState(false)
    const [saveMessage, setSaveMessage] = useState<string | null>(null)

    useEffect(() => {
        loadClasses()
    }, [])

    useEffect(() => {
        if (selectedClassId) {
            loadStudentsAndAttendance()
        }
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [selectedClassId, selectedDate])

    const loadClasses = async () => {
        try {
            const response = await window.electronAPI.class.getAll()
            if (response.success && response.data) {
                setClasses(response.data)
                if (response.data.length > 0) {
                    setSelectedClassId(response.data[0].id!)
                }
            }
        } catch (error) {
            console.error('Failed to load classes:', error)
        } finally {
            setIsLoading(false)
        }
    }

    const loadStudentsAndAttendance = async () => {
        if (!selectedClassId) return

        setIsLoading(true)
        try {
            // Load students
            const studentsResponse = await window.electronAPI.student.getByClass(selectedClassId)

            // Load existing attendance records
            const attendanceResponse = await window.electronAPI.attendance.getByClassAndDate(
                selectedClassId,
                selectedDate
            )

            if (studentsResponse.success && studentsResponse.data) {
                const attendanceMap = new Map<number, AttendanceRecord>()
                if (attendanceResponse.success && attendanceResponse.data) {
                    attendanceResponse.data.forEach(record => {
                        attendanceMap.set(record.student_id, record)
                    })
                }

                // Merge students with their attendance status
                const studentsWithAttendance: StudentWithAttendance[] = studentsResponse.data.map(student => {
                    const record = attendanceMap.get(student.id!)
                    return {
                        ...student,
                        attendanceStatus: record?.status,
                        notes: record?.notes
                    }
                })

                setStudents(studentsWithAttendance)
            }
        } catch (error) {
            console.error('Failed to load data:', error)
        } finally {
            setIsLoading(false)
        }
    }

    const updateStudentStatus = (studentId: number, status: AttendanceStatus) => {
        setStudents(prev => prev.map(s =>
            s.id === studentId ? { ...s, attendanceStatus: status } : s
        ))
    }

    const markAllAs = (status: AttendanceStatus) => {
        setStudents(prev => prev.map(s => ({ ...s, attendanceStatus: status })))
    }

    const saveAttendance = async () => {
        if (!selectedClassId) return

        setIsSaving(true)
        setSaveMessage(null)

        try {
            const recordsToSave = students
                .filter(s => s.attendanceStatus)
                .map(s => ({
                    student_id: s.id!,
                    class_id: selectedClassId,
                    date: selectedDate,
                    status: s.attendanceStatus!,
                    notes: s.notes
                }))

            for (const record of recordsToSave) {
                await window.electronAPI.attendance.record(record)
            }

            setSaveMessage('Attendance saved successfully!')
            setTimeout(() => setSaveMessage(null), 3000)
        } catch (error) {
            console.error('Failed to save attendance:', error)
            setSaveMessage('Failed to save attendance')
        } finally {
            setIsSaving(false)
        }
    }

    const getStatusBadgeClass = (status?: AttendanceStatus) => {
        switch (status) {
            case 'present': return 'bg-green-500/20 text-green-400 border-green-500/30'
            case 'absent': return 'bg-red-500/20 text-red-400 border-red-500/30'
            case 'late': return 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30'
            case 'excused': return 'bg-blue-500/20 text-blue-400 border-blue-500/30'
            default: return 'bg-gray-500/20 text-gray-400 border-gray-500/30'
        }
    }

    if (isLoading && classes.length === 0) {
        return (
            <div className="flex items-center justify-center h-full">
                <div className="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
            </div>
        )
    }

    return (
        <div className="space-y-6">
            {/* Header */}
            <div className="flex items-center justify-between">
                <div>
                    <h1 className="text-2xl font-bold text-white mb-1">Take Attendance</h1>
                    <p className="text-gray-400">Mark student attendance for the selected class and date</p>
                </div>
            </div>

            {/* Filters */}
            <div className="glass rounded-2xl p-6">
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div>
                        <label className="block text-sm text-gray-400 mb-2">Select Class</label>
                        <select
                            value={selectedClassId || ''}
                            onChange={(e) => setSelectedClassId(Number(e.target.value))}
                            className="input"
                        >
                            {classes.length === 0 && <option value="">No classes available</option>}
                            {classes.map((cls) => (
                                <option key={cls.id} value={cls.id}>
                                    {cls.name} {cls.section && `- ${cls.section}`}
                                </option>
                            ))}
                        </select>
                    </div>
                    <div>
                        <label className="block text-sm text-gray-400 mb-2">Select Date</label>
                        <input
                            type="date"
                            value={selectedDate}
                            onChange={(e) => setSelectedDate(e.target.value)}
                            className="input"
                        />
                    </div>
                    <div className="flex items-end">
                        <div className="flex gap-2 w-full">
                            <button
                                onClick={() => markAllAs('present')}
                                className="btn btn-success flex-1 text-sm"
                            >
                                All Present
                            </button>
                            <button
                                onClick={() => markAllAs('absent')}
                                className="btn btn-secondary flex-1 text-sm"
                            >
                                All Absent
                            </button>
                        </div>
                    </div>
                </div>
            </div>

            {/* Student List */}
            {students.length > 0 ? (
                <div className="glass rounded-2xl overflow-hidden">
                    <div className="p-4 border-b border-white/10 flex items-center justify-between">
                        <h2 className="font-semibold text-white">Students ({students.length})</h2>
                        {saveMessage && (
                            <span className={`text-sm ${saveMessage.includes('success') ? 'text-green-400' : 'text-red-400'}`}>
                                {saveMessage}
                            </span>
                        )}
                    </div>

                    <div className="divide-y divide-white/5">
                        {students.map((student, index) => (
                            <div
                                key={student.id}
                                className="p-4 flex items-center justify-between hover:bg-white/5 transition-colors"
                                style={{ animationDelay: `${index * 50}ms` }}
                            >
                                <div className="flex items-center gap-4">
                                    <div className="w-10 h-10 rounded-full bg-gradient-to-br from-blue-500 to-purple-500 flex items-center justify-center text-white font-medium">
                                        {student.first_name[0]}{student.last_name[0]}
                                    </div>
                                    <div>
                                        <p className="text-white font-medium">
                                            {student.last_name}, {student.first_name}
                                        </p>
                                        <p className="text-gray-500 text-sm">{student.student_id}</p>
                                    </div>
                                </div>

                                <div className="flex items-center gap-2">
                                    {(['present', 'late', 'absent', 'excused'] as AttendanceStatus[]).map((status) => (
                                        <button
                                            key={status}
                                            onClick={() => updateStudentStatus(student.id!, status)}
                                            className={`px-4 py-2 rounded-lg text-sm font-medium border transition-all ${student.attendanceStatus === status
                                                    ? getStatusBadgeClass(status)
                                                    : 'border-white/10 text-gray-500 hover:border-white/20 hover:text-gray-300'
                                                }`}
                                        >
                                            {status.charAt(0).toUpperCase() + status.slice(1)}
                                        </button>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>

                    {/* Save Button */}
                    <div className="p-4 border-t border-white/10 flex justify-end">
                        <button
                            onClick={saveAttendance}
                            disabled={isSaving}
                            className="btn btn-primary min-w-[160px]"
                        >
                            {isSaving ? (
                                <span className="flex items-center gap-2">
                                    <div className="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin"></div>
                                    Saving...
                                </span>
                            ) : (
                                'Save Attendance'
                            )}
                        </button>
                    </div>
                </div>
            ) : selectedClassId ? (
                <div className="glass rounded-2xl p-12 text-center">
                    <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-gradient-to-br from-blue-500/20 to-purple-500/20 flex items-center justify-center">
                        <svg className="w-10 h-10 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197" />
                        </svg>
                    </div>
                    <h3 className="text-xl font-semibold text-white mb-2">No Students in This Class</h3>
                    <p className="text-gray-400">Add students to this class to start taking attendance</p>
                </div>
            ) : (
                <div className="glass rounded-2xl p-12 text-center">
                    <h3 className="text-xl font-semibold text-white mb-2">Select a Class</h3>
                    <p className="text-gray-400">Choose a class from the dropdown above to take attendance</p>
                </div>
            )}
        </div>
    )
}

export default Attendance
