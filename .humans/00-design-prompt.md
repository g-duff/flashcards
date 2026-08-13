# Human prompt and design notes

## 1. Tech Spec and repo structure
I want a design for a flashcards app for language learning, inspired by DuoLingo.

When I am asking for a design, you must include:
* A database schema diagram in plaintext
* A list of user-stories
* A list of REST api routes
* Examples for how the user-stories are achieved using the proposed APIs

Tech stack:
* Front-end in `client` in typescript with web components in react. The client will primarily be used on an iPhone in the Safari browser.
    * I want these dev dependencies accessed with these package scripts. You may suggest better alternatives if they are obvious.
        * tested with vitest: `npm run test`
        * bundled with esbuild: `npm run build`
        * linted with eslint: `npm run lint`
        * formatted with prettier's default config: `npm run format`
* Back-end in `server` directory: rust
    * Web server: axum
    * Database library requirements:
        * URL must be configurable from a config file
        * config file must be yaml or json5
        * Path to migrations must be configuratble from a config file
        * Preferably using raw SQL but diesel is acceptable
    * Following the coding standards defined in "server/CODING_STANDARDS.md"

Repo structure:
* Client: Front-end concerns
* Server: Backend concerns

## Feature priorities:
* Highest: multiple choice practice. The app will show one word, and multiple possible answers, one of which is correct, then the user must select the correct translations.
    * Language practice may go in both directions eg english to spanish and spanish to english
    * Examples:
        * The app will show "hola", and 5 possible answers "hello", "apple", "brother", "eat", "day". The user must select the correct answer "hello" in order to get the question correct
        * The app will show "hello", and 5 possible answers "hola", "manzana", "hermano", "como", "bienvenido". The user must select the correct answer "hola" in order to get the question correct.
* High: vocab categories.
    * The user will select a category of vocab to practice in a session, so vocab must belong to at least one category. 
    * Categories will be user-defined and contain tens or hundreds of words.
    * One word may belong to multiple categories
* Medium: progress tracking
    * Progress tracking should support multiple users. The app should distinguish whether "George" or "Sam" is practicing, and track their progress seperately. Progress stats should be viewable by all users. The app should remember which user is using the app eg by setting cookie.
    * The app should prioritise words for practice that the user frequently gets wrong, or has not practiced for a long time. If a user frequently gets a practice question correct then that vocab word can be deprioritised.
    * The algorithm should be tunable for harshness/leniency via ui and/or config file.
    * Examples:
        * User has correctly translated "la manzana" five times today so the app won't suggest this word for practice for a few days
        * User has incorrectly translated "naranja" twice today so prioritise this word for practice for the next few days
* Low (may be deferred till later, but should be possible to extend the code to support this): text-based practice
    * The app will show a word, and the user must type the correct answer in a text box. The practice translations may go in either direction eg english to spanish, and spanish to english
    * The correct translation must include the correct article eg "el", "la".
    * Examples:
        * App shows "apple", user types in "la manzana", app responds that the user is correct
        * App shows "apple", user types in "el manzana", app responds that the user is incorrect because the article is wrong
        * App shows "apple", user types in "manzana", app responds that the user is incorrect because the article is missing
    
## UI description

The UI should have these screens:
* Home screen. Allows navigation to other screens listed below. Requires the user to enter their name or select from current users
* Choose practice cagetory. Has proficiency indicators on each category for the current user
    * redirects to home if the user is not set in the cookie
* Practice - app prompts user with practice questions. User answers. See "multiple choice practice"
    * redirects to home if the user is not set in the cookie
* Add vocab. Allows user to add words for future practice. Allows entering one or multiple vocab items for pratcie. Requires category, source and target languages.



## Development strategy
* Enable early manual user testing by prioritising completion end-to-end features, feature-by-feature if possible.
* Define a Containerfile per service, and a compose file at the repo root so that a developer can run the app for manual testing with Podman
