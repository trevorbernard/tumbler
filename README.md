# tumbler

A diceware passphrase generator with selectable entropy sources. Generates
memorable, high-entropy passphrases from the
[EFF large wordlist](https://www.eff.org/dice) and copies them directly to
your clipboard — no terminal output, no shell history.

## Usage

```
tumbler [-n <words>] [-d <device>] [-s <separator>] [-p] [-e]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-n`, `--words` | `6` | Number of words |
| `-d`, `--device` | `/dev/urandom` | Entropy source device |
| `-s`, `--separator` | `""` | Word separator |
| `-p`, `--print` | — | Print to stdout instead of clipboard |
| `-e`, `--entropy` | — | Show entropy statistics |

### Examples

```sh
# Generate a 6-word passphrase and copy to clipboard (default)
tumbler

# Print instead of copying
tumbler --print

# Use a hardware TRNG directly
tumbler --device /dev/hwrng

# 8 words with hyphens, show entropy info
tumbler --words 8 --separator - --entropy --print
# 8 words × 12.925 bits/word = 103.4 bits  (wordlist: 7776 words)
# correct-horse-battery-staple-...

# 4 words for something you'll type frequently (still ~51 bits)
tumbler --words 4 --print
```

## Security

### Entropy

tumbler uses the [EFF large wordlist](https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt)
(7776 words = 6⁵), giving **12.925 bits of entropy per word**.

| Words | Bits  | Time to crack at 10¹² guesses/sec |
|-------|-------|-----------------------------------|
| 4     | 51.7  | ~42 days                          |
| 5     | 64.6  | ~660 years                        |
| 6     | 77.5  | ~6.4 million years                |
| 7     | 90.4  | ~50 billion years                 |
| 8     | 103.4 | well beyond the heat death of the universe |

6 words is the default and is appropriate for most purposes. Use 8 for
high-value secrets or offline key material.

### Bias elimination

Index selection uses rejection sampling over `u64`. The accepted range
`[0, threshold)` is the largest multiple of the wordlist size that fits in
64 bits, ensuring every word is selected with exactly equal probability.
The rejection probability for a 7776-word list is 2624/2⁶⁴ ≈ 1.4 × 10⁻¹⁶ —
the loop body executes once in any practical scenario.

### Entropy source

By default tumbler reads from `/dev/urandom`, which on Linux 5.6+ uses a
BLAKE2s-based CSPRNG seeded from hardware entropy. This is cryptographically
strong for all practical purposes.

If you have a hardware TRNG (e.g. an RDSEED-capable CPU, a dedicated USB
device), you can pass its device path with `--device /dev/hwrng`. Note that
raw hardware RNG output is unwhitened — a defective or biased device will
produce biased passphrases. The kernel CSPRNG protects against this; bypass
it only if you have a reason to.

### Clipboard

tumbler uses [OSC 52](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Operating-System-Commands)
to copy to the clipboard. The escape sequence is written directly to `/dev/tty`
— no external tools required, no dependency on a display server, and it works
transparently over SSH.

Supported terminal emulators include kitty, foot, WezTerm, xterm (with
`allowWindowOps`), iTerm2, and tmux (with `set-clipboard on`).

If `/dev/tty` cannot be opened (fully non-interactive context), tumbler falls
back to printing with a warning.

## Building

```sh
cargo build --release
```

With Nix:

```sh
nix build
```

## Disclaimer

This software is provided as-is with no warranty. Use at your own risk. It has
not been audited by a security professional. Review the code before relying on
it for anything sensitive.

## License

MIT — see [LICENSE](LICENSE).
