import { useEffect, useState } from 'react'
import { getHealth } from './api/health'
import './App.css'

type ServerStatus = { kind: 'loading' } | { kind: 'online'; status: string } | { kind: 'offline' }

function App() {
  const [server, setServer] = useState<ServerStatus>({ kind: 'loading' })

  useEffect(() => {
    let cancelled = false

    getHealth()
      .then((data) => {
        if (!cancelled) setServer({ kind: 'online', status: data.status })
      })
      .catch(() => {
        if (!cancelled) setServer({ kind: 'offline' })
      })

    return () => {
      cancelled = true
    }
  }, [])

  return (
    <main className="app">
      <h1>Language Practice Flashcards</h1>
      <p className="server-status" data-status={server.kind}>
        {server.kind === 'loading' && 'Checking server…'}
        {server.kind === 'online' && `Server is ${server.status}`}
        {server.kind === 'offline' && 'Server is unreachable'}
      </p>
    </main>
  )
}

export default App
