# Security policy

## Supported versions

Only the latest release of Anzan receives fixes.

## Reporting a vulnerability

Please do not open a public issue. Report it privately through
[GitHub's vulnerability reporting](https://github.com/BenjaminLucier/anzan/security/advisories/new)
instead, with steps to reproduce and the version affected.

You should hear back within a week. Once a fix is released, the advisory is
published with credit to you unless you would rather stay anonymous.

Anzan runs offline and reads only its own config, theme and history files,
so the most likely problems are in parsing those files or in its
dependencies, which are audited in CI with
[cargo-deny](https://github.com/EmbarkStudios/cargo-deny).
