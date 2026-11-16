# Security policy

## Supported versions

Only the latest release of Anzan receives security fixes.

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub:
[Report a vulnerability](https://github.com/BenjaminLucier/anzan/security/advisories/new).
If you can't use GitHub, email benjaminlucier@gmail.com instead. Please don't
open a public issue or PR for a suspected vulnerability. Include steps to
reproduce and the version affected.

You can expect an acknowledgement within 3 working days. Fixes are coordinated
with you, and details are disclosed publicly once a fixed release is available,
or after 90 days at the latest. The advisory credits you unless you'd rather
stay anonymous.

## Scope

Anzan runs offline and reads only its own files. In scope: the code in this
repository, in particular

- parsing of its config, theme and history files,
- the Windows install script (`install.ps1`) and the release archives.

Out of scope:

- Vulnerabilities in dependencies that don't affect this code. Report them
  upstream; fixes are picked up through Dependabot, and dependencies are
  audited in CI with [cargo-deny](https://github.com/EmbarkStudios/cargo-deny).

## Safe harbour

Good-faith research that follows this policy, avoids accessing other people's
data, and gives us reasonable time to fix issues will not be pursued by the
maintainers.
