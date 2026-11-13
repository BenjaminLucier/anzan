# The README's screenshots and demo

The images in `docs/images/` are recorded, not taken by hand, so they can be
redone whenever the interface changes:

```sh
brew install vhs                  # also installs ttyd and ffmpeg
brew install --cask font-iosevka
demo/record.sh                    # about three minutes
```

then look over the result and commit `docs/images/`.

## How it works

[VHS](https://github.com/charmbracelet/vhs) runs a shell in a headless
browser, types what a `.tape` file says, and saves frames as PNGs or a GIF.
`record.sh`:

1. builds `anzan` and the `demo_data` example in release mode,
2. writes a fresh history for each recording with
   [`examples/demo_data.rs`](../examples/demo_data.rs): eight months of a
   keen user, the same every time apart from ending yesterday,
3. runs [`screenshots.tape`](screenshots.tape) once with each config,
   [`mocha.toml`](mocha.toml) and [`latte.toml`](latte.toml), and tiles its
   four question screenshots into `questions.png`,
4. runs [`demo.tape`](demo.tape) in Mocha for `demo.gif`.

Recordings never touch your own history: `ANZAN_DATA_DIR` and
`ANZAN_CONFIG` point at the fresh history and these configs.

The questions come from `ANZAN_SEED`, so each tape knows what will be
asked: the demo types the answers to seed 2's questions, and each question
screenshot uses a seed picked to show that kind well. If generating changes
and a seed starts asking something else, pick another; one way to see what
a seed asks is to run `ANZAN_SEED=2 anzan --duration 300`, skip a few with
Tab, and look at the end of `attempts.jsonl`.

## Why Iosevka

The large question is drawn with Unicode 16 octant characters. Ghostty and
Kitty draw those themselves, but VHS's terminal (xterm.js) takes them from
the font, and Iosevka is one of the few that has them. Even so the browser
leaves faint seams between the octants, which a real Ghostty does not.

VHS's `Screenshot` saves a frame from slightly later, after the next
command has started, so every screenshot is followed by a short `Sleep`.
