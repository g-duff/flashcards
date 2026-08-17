import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import Home from './Home'
import { createCategory, listCategories } from './api/categories'
import {
  clearLearnerSession,
  createLearner,
  deleteLearner,
  getCurrentLearner,
  listLearners,
  renameLearner,
  selectLearner,
} from './api/learners'
import { apiError } from './testUtils/envelopes'

vi.mock('./api/learners')
vi.mock('./api/categories')

const alice = {
  id: 1,
  name: 'Alice',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}
const bea = {
  id: 2,
  name: 'Bea',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}
const animals = {
  id: 1,
  name: 'Animals',
  created_at: '2026-08-12T00:00:00Z',
  updated_at: '2026-08-12T00:00:00Z',
}

describe('Home', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    // Selecting a learner mounts Categories and Add Vocabulary, both of
    // which list categories on mount; default to none so tests that don't
    // care about categories don't also need to stub this.
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [] })
    vi.mocked(listLearners).mockResolvedValue({ ok: true, value: [] })
  })

  // Home screen: display the current Learner when the cookie already
  // resolves one (spec.md story 3; ticket 02).
  it('shows the current learner when the session cookie already resolves one', async () => {
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: alice })

    render(<Home />)

    expect(await screen.findByText('Alice')).toBeInTheDocument()
  })

  // An invalid or deleted-profile cookie is cleared server-side, so
  // GET /api/session/learner resolves to null data; the client must show
  // the Home profile-choosing screen rather than getting stuck (spec.md
  // story 4; ticket 02).
  it('shows the Home profile-choosing screen when the session resolves to no learner', async () => {
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: undefined })

    render(<Home />)

    expect(await screen.findByRole('heading', { name: 'Choose a profile' })).toBeInTheDocument()
    expect(await screen.findByLabelText('New learner name')).toBeInTheDocument()
  })

  // Home screen: create a Learner and it becomes the current Learner
  // (spec.md story 1; ticket 02).
  it('lets a learner create a new profile and become the current learner', async () => {
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: undefined })
    vi.mocked(createLearner).mockResolvedValue({ ok: true, value: bea })

    render(<Home />)

    const input = await screen.findByLabelText('New learner name')
    fireEvent.change(input, { target: { value: 'Bea' } })
    fireEvent.click(screen.getByRole('button', { name: 'Create profile' }))

    expect(await screen.findByText('Bea')).toBeInTheDocument()
    expect(createLearner).toHaveBeenCalledWith('Bea')
  })

  // Home screen: select an existing Learner and it becomes the current
  // Learner (spec.md story 2; ticket 02).
  it('lets a learner select an existing profile', async () => {
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: undefined })
    vi.mocked(listLearners).mockResolvedValue({ ok: true, value: [alice] })
    vi.mocked(selectLearner).mockResolvedValue({ ok: true, value: alice })

    render(<Home />)

    const selectButton = await screen.findByRole('button', { name: 'Alice' })
    fireEvent.click(selectButton)

    expect(await screen.findByRole('button', { name: 'Switch profile' })).toBeInTheDocument()
    expect(selectLearner).toHaveBeenCalledWith(alice.id)
  })

  // Duplicate-name conflicts surface as inline feedback rather than
  // silently failing (spec.md story 6; ticket 02).
  it('shows an error when creating a learner with a duplicate name', async () => {
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: undefined })
    vi.mocked(createLearner).mockResolvedValue({
      ok: false,
      error: apiError('A learner with this name already exists.', [
        { field: 'name', reason: 'Already in use' },
      ]),
    })

    render(<Home />)

    const input = await screen.findByLabelText('New learner name')
    fireEvent.change(input, { target: { value: 'Alice' } })
    fireEvent.click(screen.getByRole('button', { name: 'Create profile' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'A learner with this name already exists.',
    )
  })

  // A Learner can rename their profile without losing identity (spec.md
  // story 5; ticket 03).
  it('lets the current learner rename their profile', async () => {
    const renamed = { ...alice, name: 'Alicia' }
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: alice })
    vi.mocked(renameLearner).mockResolvedValue({ ok: true, value: renamed })

    render(<Home />)

    const input = await screen.findByLabelText('Rename profile')
    fireEvent.change(input, { target: { value: 'Alicia' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save name' }))

    expect(await screen.findByText('Alicia')).toBeInTheDocument()
    expect(renameLearner).toHaveBeenCalledWith(1, 'Alicia')
  })

  // A Learner can deliberately delete their profile, so their personal data
  // is removed (spec.md story 7; ticket 03).
  it('lets the current learner delete their profile after confirming', async () => {
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: alice })
    vi.mocked(deleteLearner).mockResolvedValue({ ok: true, value: undefined })

    render(<Home />)

    fireEvent.click(await screen.findByRole('button', { name: 'Delete profile' }))
    fireEvent.click(screen.getByRole('button', { name: 'Confirm delete' }))

    expect(await screen.findByRole('heading', { name: 'Choose a profile' })).toBeInTheDocument()
    expect(deleteLearner).toHaveBeenCalledWith(1)
  })

  // Cancelling the delete confirmation leaves the profile untouched.
  it('lets the current learner cancel a delete confirmation', async () => {
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: alice })

    render(<Home />)

    fireEvent.click(await screen.findByRole('button', { name: 'Delete profile' }))
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(screen.queryByRole('button', { name: 'Confirm delete' })).not.toBeInTheDocument()
    expect(createLearner).not.toHaveBeenCalled()
    expect(selectLearner).not.toHaveBeenCalled()
    expect(renameLearner).not.toHaveBeenCalled()
    expect(deleteLearner).not.toHaveBeenCalled()
    expect(clearLearnerSession).not.toHaveBeenCalled()
  })

  // Categories and Add Vocabulary each fetch their own category list, so
  // creating a category must signal Add Vocabulary to refetch rather than
  // leaving its checkboxes stale until a page reload.
  it('shows a newly created category in Add Vocabulary without reloading', async () => {
    vi.mocked(getCurrentLearner).mockResolvedValue({ ok: true, value: alice })
    vi.mocked(createCategory).mockResolvedValue({ ok: true, value: animals })

    render(<Home />)

    await screen.findByText('Alice')
    expect(screen.queryByLabelText('Animals')).not.toBeInTheDocument()

    // The refresh triggered by a successful create refetches the list.
    vi.mocked(listCategories).mockResolvedValue({ ok: true, value: [animals] })

    fireEvent.change(screen.getByLabelText('New category name'), { target: { value: 'Animals' } })
    fireEvent.click(screen.getByRole('button', { name: 'Create category' }))

    expect(await screen.findByLabelText('Animals')).toBeInTheDocument()
  })
})
