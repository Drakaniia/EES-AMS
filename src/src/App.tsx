import { useState } from 'react'
import './index.css'
import { AuthProvider, useAuth } from './contexts/AuthContext'
import Sidebar from './components/Sidebar'
import UpdateNotification from './components/UpdateNotification'
import Dashboard from './pages/Dashboard'
import Attendance from './pages/Attendance'
import Classes from './pages/Classes'
import Students from './pages/Students'
import Settings from './pages/Settings'
import AuthScreen from './pages/AuthScreen'

type Page = 'dashboard' | 'attendance' | 'classes' | 'students' | 'settings'

function AppContent() {
  const { userProfile, loading } = useAuth()
  const [currentPage, setCurrentPage] = useState<Page>('dashboard')

  const renderPage = () => {
    switch (currentPage) {
      case 'dashboard':
        return <Dashboard />
      case 'attendance':
        return <Attendance />
      case 'classes':
        return <Classes />
      case 'students':
        return <Students />
      case 'settings':
        return <Settings />
      default:
        return <Dashboard />
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="flex flex-col items-center gap-4">
          <div className="w-12 h-12 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
          <p className="text-gray-400">Loading...</p>
        </div>
      </div>
    )
  }

  if (!userProfile) {
    return <AuthScreen />
  }

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Custom title bar drag region */}
      <div
        className="fixed top-0 left-0 right-0 h-10 z-50"
        style={{ WebkitAppRegion: 'drag' } as React.CSSProperties}
      />

      {/* Sidebar */}
      <Sidebar currentPage={currentPage} onNavigate={setCurrentPage} />

      {/* Main Content */}
      <main className="flex-1 overflow-auto pt-10 pb-6 px-6">
        <div className="animate-fade-in">
          {/* Update Notification */}
          <UpdateNotification />
          
          {renderPage()}
        </div>
      </main>
    </div>
  )
}

function App() {

return (
    <AuthProvider>
      <AppContent />
    </AuthProvider>
  )
}

export default App
