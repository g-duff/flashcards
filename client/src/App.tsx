import { useEffect, useState } from 'react'
import { getHealth } from './api/health'
import Home from './Home'
import './App.css'

type ServerStatus = { kind: 'loading' } | { kind: 'online'; status: string } | { kind: 'offline' }

function App() {
  const [server, setServer] = useState<ServerStatus>({ kind: 'loading' })

  useEffect(() => {
    let cancelled = false

    getHealth().then((result) => {
      if (cancelled) return
      setServer(result.ok ? { kind: 'online', status: result.value.status } : { kind: 'offline' })
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
      <Home />
    </main>
  )
}

export default App
