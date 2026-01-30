# zdoc
A Rust documentation parser.

## What this is
I always found `cargo doc` frustrating. I mean, the command is RIGHT THERE.
What if I want to fuzzy find a search query inline instead of open a web browser?

Just rolling with the vision and seeing what emerges.

## How it (would) work
`zdoc` should only run inside a valid Rust project. It doesn't exist in a global context.
Why would you be searching versioned Rust crate docs from your CLI outside of a Rust project? Lol.

Makes the constraint surface for the search functionality dead simple.
Just looking at what crates are actually present in the local `Cargo.toml` file.

> "Error: No `Cargo.toml` found. `zdoc` is designed to run within a Rust project to provide version-accurate documentation."
User will know what this means since... they've likely installed it via Cargo itself.

## Commands (proposed)
- `search [query] [crate] {--results N}`
    - Returns the top N scored fuzzy results for the given query in the given crate.
        - *(defaults to global search of all installed deps if no crate specified)*
    - Shows the function signature and the first line of the doc comment if present.
        - i.e, `fn execute(...) -> Result<T> ([doc comment])`

- `diff [crate] [ver1] [ver2]`
    - Pulls docs for both versions and shows a terminal diff of the public API.
    - Output is similar to `git diff`, additions are green and removals are red.

- `features [crate]`
    - A quick way to list the available features for the provided crate.
    - Maybe shows a simple ASCII tree to show dependency trees?