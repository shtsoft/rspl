# Contributing to rspl

Contributions are welcome!
But please read this guide before.

## Code

If you plan to contribute code please take the following subsections into account.

### Implementation

Pay attention to security, usability, maintainability and performance (roughly in that order).
Generally, try to adhere to the coding-style of the current implementation to keep reading the code easy.
Specifically,
- make sure that the code is formatted correctly by running `cargo fmt --all -- --check`.
- make sure that the linter does not complain too much by running `cargo clippy --all --benches --examples --tests --all-features`.

### Documentation

Document everything adhering to the current style.
In particular, comment private functions.
In the end, make sure that the docs build by running `cargo doc --no-deps --document-private-items`.

### Testing

If you add code then also add tests as necessary.
Both integration- and unit-tests.
It is also suggested to include one documentation-test for each public API function.
In the end, make sure that all tests work as expected by running `cargo test`.

## Github

This project is hosted on Github.
In the folllowing it is briefly described how this project intends to leverage Github for development.

### Issues

Github Issues is how contributors shall communicate.
Issue-templates are provided to type communication a bit.
But in doubt, feel free to use blank issues.

### Workflows

When making a contribution keep an eye on the [workflows](.github/workflows).

In a nutshell, what they do is

- Audit
  * checking for known bugs in dependencies
- CI
  * checking if tests run
  * checking for memory leak
  * checking format
  * checking for lints
  * checking documentation

### Pull Requests

As usual, if you can not directly push to the repository use pull requests for contributions.
A pull request-template is provided to assist you.
Otherwise try to keep pull request small and focused (like a topic-branch - see [Git](#git)).
In any case, take all the other sections into account to ease the pain of merging your pull request.

## Git

This project uses git for version control and contributors are kindly asked to use git as described in the [git-book](https://git-scm.com/book/en/v2).
In the following the most important aspects are recounted and few things are nailed down.

### Commits

Commit in 'sensible' units.
In particular, do NOT
- misuse commits for your backup-strategy.
- rewrite your commit history after pushing.

Moreover run `git diff --check` before a commit to check for whitespace errors.

#### Messages

A commit message is written in the imperative (e.g.: 'Fix bug' instead of 'Fixed bug').
The first line of a commit message is a short description of the changeset in less than 50 characters.
Optionally, it can be followed by a blank second line and a more detailed explanatory text starting at the third line.
The text should i.p. motivate the change and explain the difference.

### Branching

This project's branching workflow is 'progressive-stability branching'.
This means that there is a permanent `development`-branch (organizing development) in addition to the `master`-branch.
`development` is merged into master whenever it is in a stable state.
Actual development takes place in so-called topic-branches (e.g. `iss4` to handle the fourth issue) emanating from `development`.
The topic-branches are merged back and deleted when the topic is treated.

## Versioning

This project uses [Semantic Versioning](https://semver.org) - read that if you do not already know it.
When `development` is in a state such that [Semantic Versioning](https://semver.org) requires a version-action then edit the `version`-field of [Cargo.toml](Cargo.toml) accordingly and make a commit with message 'Release version X.Y.Z'.
Check if publishing would work with `cargo publish --dry-run`.
Afterwards tag the commit (`git tag vX.Y.Z -m 'Release version X.Y.Z'`) and merge development back into master.
Finally, make a release on Github and publish on crates.io.
