# React Core Concepts Through `Home.tsx`

This guide explains the main React ideas used by
[`client/src/Home.tsx`](../client/src/Home.tsx).

## The central mental model

React lets you describe:

> Given the current data, what should the screen look like?

```text
props / state
      |
      v
component function
      |
      v
     JSX
      |
      v
   browser UI
```

The component does not usually manipulate the DOM directly. Instead, it
updates state, and React calculates the appropriate UI.

## 1. Components

A component is a reusable piece of UI. `Home.tsx` defines one:

```tsx
const Home = () => {
  return <h1>Welcome</h1>
}
```

A component can contain JSX, JavaScript logic, state, event handlers, and
child components.

```text
input data -> component -> displayed UI
```

## 2. JSX

JSX looks like HTML but is written inside JavaScript or TypeScript:

```tsx
return (
  <section>
    <h2>Choose a profile</h2>
    <p>Current learner: {name}</p>
  </section>
)
```

Expressions go inside braces:

```tsx
<p>{learner.name}</p>
```

JSX is a description of the UI. React uses that description to update the
browser.

## 3. Props

Props are values passed from a parent component to a child component:

```tsx
function LearnerCard({ name }: { name: string }) {
  return <p>{name}</p>
}

<LearnerCard name="Alice" />
```

Data normally flows in one direction:

```text
Parent
  |
  | props
  v
Child
```

Props are read-only from the child’s perspective. If a child needs to cause a
change, the parent can pass a callback.

## 4. State

State is data owned by a component that can change:

```tsx
const [newName, setNewName] = useState('')
```

This provides:

- `newName`: the current value
- `setNewName`: the function used to change it
- `''`: the initial value

When the setter is called, React renders the component again:

```text
setNewName("Bea")
        |
        v
React renders Home again
        |
        v
value={newName} becomes "Bea"
        |
        v
input displays "Bea"
```

Do not directly modify state. Use its setter:

```tsx
setNewName('Bea')
```

## 5. Rendering as a calculation

A useful approximation is:

```text
UI = f(props, state)
```

`Home.tsx` has three pieces of state:

```tsx
const [screen, setScreen] = useState<Screen>({ kind: 'loading' })
const [newName, setNewName] = useState('')
const [feedback, setFeedback] = useState<Feedback>({ kind: 'idle' })
```

```text
screen state ------+
newName state -----+--> Home --> JSX/UI
feedback state ----+
```

Changing one of these values causes React to calculate the UI again.

## 6. React’s render cycle

Most interactions follow this cycle:

```text
1. Initial state
        |
        v
2. React renders the component
        |
        v
3. Browser displays the UI
        |
        v
4. User clicks or types
        |
        v
5. Event handler runs
        |
        v
6. State setter is called
        |
        v
7. React renders again
        |
        v
8. Browser updates what changed
```

React normally updates the relevant DOM rather than reloading the whole page.

## 7. Events

React event handlers respond to user actions:

```tsx
<button onClick={handleSwitchProfile}>
  Switch profile
</button>
```

When the button is clicked, React calls `handleSwitchProfile`.

Forms use `onSubmit`:

```tsx
<form onSubmit={handleCreate}>
```

The handler prevents the browser’s traditional full-page form submission:

```tsx
async function handleCreate(event: SubmitEvent<HTMLFormElement>) {
  event.preventDefault()
}
```

## 8. Controlled inputs

This input is controlled by React:

```tsx
<input
  value={newName}
  onChange={(event) => setNewName(event.target.value)}
/>
```

The flow is:

```text
User types "Bea"
       |
       v
onChange runs
       |
       v
setNewName("Bea")
       |
       v
React renders again
       |
       v
value becomes "Bea"
```

React is the source of truth for the input. This makes validation, clearing,
disabling buttons, and displaying the value elsewhere straightforward.

## 9. Effects and API requests

Rendering should mainly calculate UI. API requests are side effects, so
`Home.tsx` performs its initial request inside `useEffect`:

```tsx
useEffect(() => {
  loadInitialScreen()
}, [])
```

The empty dependency array means the effect runs after the component first
appears.

```text
Home appears
    |
    v
useEffect runs
    |
    v
API request
    |
    v
setScreen(...)
    |
    v
Home renders with loaded data
```

The effect first checks for a current learner. If none exists, it loads the
available learner profiles.

The cleanup function protects against updating state after unmounting:

```tsx
return () => {
  cancelled = true
}
```

## 10. Conditional rendering

React uses normal JavaScript conditions to decide what to display.

`Home.tsx` uses `if` statements for whole screens:

```tsx
if (screen.kind === 'loading') {
  return <p>Loading your profile...</p>
}
```

It uses `&&` for optional content:

```tsx
{screen.learners.length > 0 && (
  <ul>...</ul>
)}
```

The list appears only when there is at least one learner.

A ternary is useful for choosing between two small alternatives:

```tsx
{isLoggedIn ? <Dashboard /> : <Login />}
```

## 11. Lists and keys

Learners are converted into list items with `.map()`:

```tsx
screen.learners.map((learner) => (
  <li key={learner.id}>
    <button onClick={() => handleSelect(learner.id)}>
      {learner.name}
    </button>
  </li>
))
```

```text
[
  Alice,
  Bea
]
    |
    v
<li>Alice</li>
<li>Bea</li>
```

The `key` gives each item a stable identity so React can update lists
correctly. Stable IDs are preferable to array indexes.

## 12. Modeling UI states

`Home.tsx` represents its main screens with a discriminated union:

```tsx
type Screen =
  | { kind: 'loading' }
  | { kind: 'selected'; learner: Learner }
  | { kind: 'choosing'; learners: Learner[] }
```

The possible transitions are:

```text
loading
   |
   +-- learner found ------> selected
   |
   +-- no learner ----------> choosing

choosing
   |
   +-- create learner ------> selected
   |
   +-- select learner ------> selected

selected
   |
   +-- switch profile ------> choosing
```

This is safer than using several independent booleans such as
`isLoading`, `hasLearner`, and `isChoosing`, which could describe contradictory
states. The `kind` field makes the valid states explicit.

Feedback uses the same idea:

```tsx
type Feedback =
  | { kind: 'idle' }
  | { kind: 'submitting' }
  | { kind: 'error'; message: string }
```

## 13. Async user flows

Creating a profile follows this sequence:

```text
submit form
    |
    v
feedback = submitting
    |
    v
call createLearner(newName)
    |
    +-- success --> show selected learner
    |
    +-- failure --> show error message
```

The UI disables the submit button while the request is running:

```tsx
disabled={feedback.kind === 'submitting'}
```

Errors are displayed only when the state represents an error:

```tsx
{feedback.kind === 'error' && (
  <p role="alert">{feedback.message}</p>
)}
```

The `alert` role helps assistive technology announce the message.

## The key React principle

Instead of manually instructing the browser:

```text
Find this paragraph.
Change its text.
Hide this button.
Create this list item.
```

React code describes the desired result:

```text
If screen.kind is "selected":
    show the current learner.

If screen.kind is "choosing":
    show the profile-selection form.
```

The practical rule to remember is:

> Change the data; let React calculate the interface.
