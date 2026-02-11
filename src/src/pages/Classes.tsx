import { FC, useEffect, useState } from 'react'
import type { Class } from '../lib/tauri'

const Classes: FC = () => {
    const [classes, setClasses] = useState<Class[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [showModal, setShowModal] = useState(false)
    const [formData, setFormData] = useState({ name: '', section: '', school_year: '' })
    const [isSubmitting, setIsSubmitting] = useState(false)

    useEffect(() => {
        loadClasses()
    }, [])

    const loadClasses = async () => {
        try {
            const response = await window.electronAPI.class.getAll()
            if (response.success && response.data) {
                setClasses(response.data)
            }
        } catch (error) {
            console.error('Failed to load classes:', error)
        } finally {
            setIsLoading(false)
        }
    }

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault()
        if (!formData.name.trim()) return

        setIsSubmitting(true)
        try {
            const response = await window.electronAPI.class.create({
                name: formData.name.trim(),
                section: formData.section.trim() || undefined,
                school_year: formData.school_year.trim() || undefined
            })

            if (response.success) {
                setFormData({ name: '', section: '', school_year: '' })
                setShowModal(false)
                loadClasses()
            }
        } catch (error) {
            console.error('Failed to create class:', error)
        } finally {
            setIsSubmitting(false)
        }
    }

    const handleDelete = async (id: number) => {
        if (!confirm('Are you sure you want to delete this class?')) return

        try {
            const response = await window.electronAPI.class.delete(id)
            if (response.success) {
                loadClasses()
            }
        } catch (error) {
            console.error('Failed to delete class:', error)
        }
    }

    if (isLoading) {
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
                    <h1 className="text-2xl font-bold text-white mb-1">Classes</h1>
                    <p className="text-gray-400">Manage your classes and sections</p>
                </div>
                <button onClick={() => setShowModal(true)} className="btn btn-primary">
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                    </svg>
                    Add Class
                </button>
            </div>

            {/* Classes Grid */}
            {classes.length > 0 ? (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                    {classes.map((cls) => (
                        <div key={cls.id} className="glass rounded-2xl p-6 card-hover group">
                            <div className="flex items-start justify-between mb-4">
                                <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center">
                                    <svg className="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
                                    </svg>
                                </div>
                                <button
                                    onClick={() => handleDelete(cls.id!)}
                                    className="opacity-0 group-hover:opacity-100 transition-opacity p-2 hover:bg-red-500/20 rounded-lg"
                                >
                                    <svg className="w-5 h-5 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                    </svg>
                                </button>
                            </div>

                            <h3 className="text-xl font-semibold text-white mb-1">{cls.name}</h3>
                            {cls.section && (
                                <p className="text-gray-400 text-sm mb-2">Section: {cls.section}</p>
                            )}
                            {cls.school_year && (
                                <span className="badge badge-success">{cls.school_year}</span>
                            )}
                        </div>
                    ))}
                </div>
            ) : (
                <div className="glass rounded-2xl p-12 text-center">
                    <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-gradient-to-br from-blue-500/20 to-purple-500/20 flex items-center justify-center">
                        <svg className="w-10 h-10 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4" />
                        </svg>
                    </div>
                    <h3 className="text-xl font-semibold text-white mb-2">No Classes Yet</h3>
                    <p className="text-gray-400 mb-6">Create your first class to get started</p>
                    <button onClick={() => setShowModal(true)} className="btn btn-primary">
                        Create Your First Class
                    </button>
                </div>
            )}

            {/* Modal */}
            {showModal && (
                <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
                    <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={() => setShowModal(false)}></div>
                    <div className="glass rounded-2xl p-6 w-full max-w-md relative animate-fade-in">
                        <h2 className="text-xl font-semibold text-white mb-6">Add New Class</h2>

                        <form onSubmit={handleSubmit} className="space-y-4">
                            <div>
                                <label className="block text-sm text-gray-400 mb-2">Class Name *</label>
                                <input
                                    type="text"
                                    value={formData.name}
                                    onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                                    placeholder="e.g., Grade 10"
                                    className="input"
                                    required
                                />
                            </div>

                            <div>
                                <label className="block text-sm text-gray-400 mb-2">Section</label>
                                <input
                                    type="text"
                                    value={formData.section}
                                    onChange={(e) => setFormData({ ...formData, section: e.target.value })}
                                    placeholder="e.g., Section A"
                                    className="input"
                                />
                            </div>

                            <div>
                                <label className="block text-sm text-gray-400 mb-2">School Year</label>
                                <input
                                    type="text"
                                    value={formData.school_year}
                                    onChange={(e) => setFormData({ ...formData, school_year: e.target.value })}
                                    placeholder="e.g., 2025-2026"
                                    className="input"
                                />
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
                                    {isSubmitting ? 'Creating...' : 'Create Class'}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            )}
        </div>
    )
}

export default Classes
