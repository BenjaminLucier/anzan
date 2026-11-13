# Anzan 暗算

A terminal trainer for mental math.

Anzan asks a stream of questions against the clock: arithmetic, powers and
roots, number theory, fractions and percentages, and finance. Every answer
is saved. The history powers a statistics dashboard, and practice modes that
find your weak spots and work on them.

```text
 1:52                           Score 7                        Esc menu

                                8 × 39

                        ┌──────────────────────┐
                        │          31          │
                        └──────────────────────┘

                                Tab skip
```

## The name

**暗算** (*anzan*) is Japanese for mental calculation. 暗 (*an*) means "dark"
or "unseen", and 算 (*san*) means "to calculate": arithmetic done without
anything written down. The word is best known from soroban (abacus) training,
where advanced students drop the physical abacus and move its beads in their
heads, adding long columns of numbers in seconds.

The practice modes take their names from Japanese too; see [Modes](#modes).

## Install

With a Rust toolchain (1.88 or later):

```sh
cargo install anzan
```

Or from a checkout of this repository, `cargo install --path .`.

## Use

Run `anzan` to open the menu: a panel of practice modes, one for the
session's length and submit mode, and one for each group of problems.
Choose what to practise, then press Enter.

| Key              | Menu                        | Practice     | Dashboard          |
|------------------|-----------------------------|--------------|--------------------|
| ← → or h l, Tab  | switch panel                |              | switch list        |
| ↑ ↓ or j k       | move; in Mode, choose mode  |              | select             |
| 1–8              | jump to a panel             |              | jump to a list     |
| Space            | toggle or change            |              |                    |
| a                | toggle the whole group      |              |                    |
| Enter            | start                       | submit       |                    |
| Tab              |                             | skip         |                    |
| PgUp PgDn, J K   |                             |              | scroll details     |
| d                | open the dashboard          |              |                    |
| r                | undo changes made here      |              |                    |
| t                | next colour theme           |              | next colour theme  |
| X                |                             |              | delete all history |
| ?                | key help                    |              | key help           |
| Esc              | quit                        | back to menu | back to menu       |

Any flag starts a session straight away, skipping the menu. The flags stay
in effect afterwards, so when time is up you land in the menu with the same
settings:

```sh
anzan --duration 60 --kinds mul,div
anzan --mode oni -k add,sub,mul,div
anzan --submit enter
```

### Answering

With `submit = "auto"`, the default, a correct answer moves on as soon as it
is typed; there is no need to press Enter. A wrong answer is never final,
only slow, though Enter submits one on purpose. With `submit = "enter"`,
nothing counts until Enter, so wrong answers count as wrong. After a miss or
a skip, the question and its answer appear under the box.

Answers can be written however is natural:

- Numbers: `0.125`, `.125` and `1/8` are all the same. `$`, `%` and `≈` are
  ignored, and so are thousands separators.
- Fractions, when asked for: in lowest terms, such as `3/8`.
- Yes or no: `y` or `n`, which also submits straight away.
- Prime factors: in any order, as `2 2 2 3`, `2*2*2*3` or `2^3 3`.
- Approximations, such as square roots: to the places asked, give or take
  one in the last place.

## Problems

| Group                     | Kind (`--kinds` name)                                                    |
|---------------------------|--------------------------------------------------------------------------|
| Arithmetic                | `add`, `sub`, `mul`, `div`, `long_mul`                                   |
| Powers and roots          | `square`, `cube`, `power_of_two`, `sqrt` (to two places)                 |
| Number theory             | `divisible`, `prime`, `factorize`                                        |
| Fractions and percentages | `convert` (fraction ↔ decimal ↔ percent), `percent_of`, `percent_change`, `successive` (+20% then −10%) |
| Finance                   | `bps` (basis points), `compound` (growth over years), `rule_of_72` (doubling time) |
| Probability               | `dice` (two dice totalling n), `coins` (k heads in n flips), `expected_value` (of a bet) |

The default is the four basic operations: addition and subtraction up to
100 + 100, and multiplication and division up to 12 × 100.

Questions are written in your terminal's own font, with exponents as
superscripts, fractions stacked over a bar and roots under an overline.
In a terminal that can scale text (Kitty and Ghostty, through the text
sizing protocol), the question is drawn at twice the size; Anzan asks the
terminal at startup, and elsewhere draws it at the usual size.

## Modes

Every mode asks the same kinds of problem; they differ in which ones. All
but Jiyū learn from your history: each attempt is scored by how much slower
it was than your usual time for that kind and size of problem, so 12 × 97
is compared with other two-digit products rather than with 2 × 3, with
extra for misses, skips and backspaces. Those scores are averaged over
skills: a single question, a times-table fact, multiplying by 7, a number
of carries, or a question with a shortcut to spot, such as 47 × 53
(50² − 3²) or 16 × 35 (8 × 70).

| Mode               | Meaning     | Problems                                                 |
|--------------------|-------------|----------------------------------------------------------|
| **Jiyū** 自由      | freedom     | At random.                                               |
| **Kaizen** 改善    | improvement | At random, leaning gently toward weak spots.             |
| **Keiko** 稽古     | practice    | New problems built on your weak skills.                  |
| **Fukushū** 復習   | review      | Questions that gave you trouble, asked again.            |
| **Oni** 鬼         | demon       | Relentlessly on your weakest skills.                     |
| **Rakushō** 楽勝   | easy win    | Questions you know well, to warm up.                     |
| **Custom**         |             | Off until `source` or `focus` is set under `[drill]`.    |

With no history yet, every mode asks problems at random.

## Statistics

Press `d` on the menu or after a session. Lists run down the left, and the
right shows the details of whatever is selected:

- **Summary**: totals and your daily streak, with the score trend over
  sessions with exactly the current settings (scores are only comparable
  then), practice per day over the past year, and response time by
  percentile, over all time and the last week.
- **Kinds**: accuracy and median time for each kind practised. Select one
  for its speed curve, and beneath it what each of its skills costs you in
  time (carrying twice, multiplying by 7, numbers ending in 8 or 9), its
  hardest questions, and for multiplication and division a times-table
  heatmap running from easier than your average to harder.
- **Sessions**: every session, newest first. Select one to see each
  question asked, the answer, what you gave and how long it took.
- **Weak spots**: the questions and the skills that take you more effort
  than your average, worst first. A question is listed once it has been
  asked twice, and a skill once it has come up five times. Select one to
  see every attempt behind it and, for a question with a shortcut, what
  the shortcut is.

Times are compared only between problems of the same kind and size, and
by default only within the same submit mode, since pressing Enter takes
time of its own.

## Configuration

Settings live in `$XDG_CONFIG_HOME/anzan/config.toml` (usually
`~/.config/anzan/config.toml`, on macOS too). Every setting is optional.

```sh
anzan config init   # write a commented file holding every default
anzan config show   # print the settings in effect
anzan config path   # print where the file is read from
```

The file covers the session (length, submit mode, practice mode), the
ranges each kind of problem draws from, the custom mode, how attempts
are scored, and the colour theme. Scoring is recomputed from the raw history every time, so
changing it rescores the past as well. Unknown keys are errors, so a typo
never goes unnoticed.

Changes made in the menu last until Anzan exits and are never written back;
r in the menu undoes them all.

## Themes

By default Anzan asks the terminal whether its background is light or dark,
then uses [Catppuccin](https://catppuccin.com) Latte or Mocha to match. If
the terminal does not say, it falls back to `terminal`, which uses the
terminal's own sixteen colours so that whatever theme it has shows through.

```toml
[theme]
name = "auto"               # or "terminal", or any theme's name
light = "catppuccin-latte"  # what auto picks on a light background
dark = "catppuccin-mocha"   # and on a dark one
```

Bundled: the four Catppuccin flavours, Dracula, Rosé Pine and Rosé Pine
Dawn, and Solarized dark and light. Their licenses are in
[THEMES.md](THEMES.md).

For more, drop any base16 or base24 scheme from
[tinted-theming/schemes](https://github.com/tinted-theming/schemes) into a
`themes` folder beside the config file; the file's name becomes the
theme's name. `anzan themes` lists everything on offer and where that
folder is. Press `t` in the menu or dashboard to try each in turn; like
other menu changes, this lasts until Anzan exits.

Palettes are drawn in true colour when `COLORTERM` says the terminal
supports it, and in the nearest of 256 colours otherwise.

## History

Every answer is appended to `$XDG_DATA_HOME/anzan/attempts.jsonl` (usually
`~/.local/share/anzan`), and every session to `sessions.jsonl` beside it:
one JSON object per line, easy to read with other tools. To export as CSV:

```sh
anzan export attempts > attempts.csv
anzan export sessions -o sessions.csv
```

To start over, press `X` (capital) in the dashboard. It says what will be
deleted and asks first; only `y` goes ahead, and both files are then
removed for good, so export first if you may want them back.

`ANZAN_CONFIG` and `ANZAN_DATA_DIR` override the two locations.

## Inspiration

Anzan is inspired by [Zetamac](https://arithmetic.zetamac.com), the
minimal timed arithmetic game, and uses its settings as the defaults for basic
arithmetic.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.
