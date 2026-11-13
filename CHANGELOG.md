# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-13

### Added

- Progress over time in the statistics dashboard: a weekly chart of your
  overall effort in the summary, median seconds per week for each kind, and
  effort per week for each weak spot.
- `ANZAN_SEED` makes a session ask the same questions every time.

### Documentation

- Screenshots and a demo in the README, recorded with VHS from generated
  history in Catppuccin Mocha and Latte.

## [0.1.1] - 2026-11-13

### Fixed

- Download the right checksum file in the Windows install script

## [0.1.0] - 2026-11-13

The first release.

### Added

- Timed sessions of arithmetic, powers and roots, number theory, fractions
  and percentages, finance and probability, each kind with configurable
  ranges.
- Practice modes that draw fresh, similar or past questions and focus on
  weak spots: Jiyū, Kaizen, Keiko, Fukushū, Oni, Rakushō, and a custom mode.
- Answers checked as they are typed, or on Enter, accepting fractions,
  decimals, percentages and other natural forms.
- A history of every answer and session, kept as JSON lines and exportable
  as CSV.
- A statistics dashboard: score trends, response times, times-table and
  activity heatmaps, sessions, and weak spots down to the skills behind them.
- Colour themes that follow a light or dark terminal, the bundled Catppuccin,
  Dracula, Rosé Pine and Solarized palettes, and any base16 or base24 scheme.
- A configuration file layered under command-line flags and menu changes.
