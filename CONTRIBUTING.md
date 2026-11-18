# Contributing to Anzan

Thanks for your interest! Bug reports, ideas and pull requests are all
welcome. For anything larger than a small fix, please open an issue first so
we can agree on the approach before you spend time on it.

By taking part you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).

## Building

You need Rust 1.88 or later ([rustup](https://rustup.rs)).

```sh
cargo run                 # the menu
cargo run -- --kinds mul  # straight into a session
```

`cargo run` keeps its config and history in `target/dev`
([.cargo/config.toml](.cargo/config.toml)), so trying things out never
touches the history of an installed Anzan. `cargo clean` wipes it. To try a
build on your real history, run it outside cargo:

```sh
cargo build --release && target/release/anzan
```

## Before opening a pull request

CI runs these, and all must pass:

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo deny check   # if you changed dependencies; needs cargo-deny
```

## Commit messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org/),
since the changelog is generated from them:

```text
feat: add a mode for squares of two-digit numbers
fix: accept a leading plus sign in answers
docs: explain the struggle score
```

Use `feat` for new behaviour, `fix` for bugs, and `docs`, `refactor`,
`test`, `perf`, `ci` or `chore` for the rest. Mark breaking changes with `!`,
as in `feat!: ...`. Keep each commit to one change and describe why, not
just what.

## License

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in Anzan by you, as defined in the Apache-2.0
license, shall be dual licensed under MIT OR Apache-2.0, without any
additional terms or conditions.

## Releasing

For the maintainer. Versions follow [Semantic Versioning](https://semver.org),
and releases are automated with [release-plz](https://release-plz.dev) from
the commit messages on `main`.

1. Each push to `main` that includes a `feat`, `fix` or `perf` commit opens
   or updates a release PR. It bumps `version` in `Cargo.toml` and
   `Cargo.lock` and adds the new `CHANGELOG.md` section. The PR is
   regenerated on every push, so make any edits to its changelog just
   before merging.
2. Merge the release PR.

The [release workflow](.github/workflows/release.yml) takes it from there.
It publishes the crate to crates.io, tags the commit (`vX.Y.Z`) and creates
a draft GitHub Release, builds the binaries into it, publishes the release,
updates the [Homebrew tap](https://github.com/BenjaminLucier/homebrew-tap),
and installs the result through Homebrew and the Windows script to check
it. A pre-release version, such as `1.2.0-rc.1`, skips Homebrew. The crate
is published before the binaries build, so if a build fails, re-run the
failed jobs; if the code itself is broken, fix it in a patch release and
delete the stale draft.
