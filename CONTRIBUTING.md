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

To keep your real history clean while testing, point Anzan somewhere else:

```sh
ANZAN_DATA_DIR=/tmp/anzan ANZAN_CONFIG=/tmp/anzan.toml cargo run
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

For the maintainer. Versions follow [Semantic Versioning](https://semver.org).

1. Add the new section to the changelog, then edit it to read well:

   ```sh
   git cliff --unreleased --tag vX.Y.Z --prepend CHANGELOG.md
   ```

   ([git-cliff](https://git-cliff.org): `brew install git-cliff`.)
2. Set `version` in `Cargo.toml` and run `cargo check` to update
   `Cargo.lock`.
3. Commit as `chore: release vX.Y.Z` and merge it to `main`.
4. Tag that commit and push the tag:

   ```sh
   git tag -a vX.Y.Z -m vX.Y.Z
   git push origin vX.Y.Z
   ```

The [release workflow](.github/workflows/release.yml) takes it from there.
It checks the tag against `Cargo.toml` and the changelog, then builds the
binaries into a draft GitHub Release, publishes the crate to crates.io,
publishes the release, updates the
[Homebrew tap](https://github.com/BenjaminLucier/homebrew-tap), and
installs the result through Homebrew and the Windows script to check it.
A tag with a pre-release suffix, such as `v1.2.0-rc.1`, skips Homebrew.
