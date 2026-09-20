# common

What the `tect` CLI and the bootc installer both use. The crate depends on
neither, so the dependency runs one way.

    ui       the terminal widgets both draw with
    json     the JSON writer every emitted document is built from
    prompt   what no flag gave, asked on a terminal and named when there is none

## Using it

Consumers pin it by commit:

    common = { git = "https://github.com/tectonic-os/common", rev = "<sha>" }

No version is published. A change here is a commit and a pin bump in each
consumer, which is what keeps the rev that was built with visible in the
consumer's `Cargo.lock`.

ratatui and libc are the whole dependency floor, and `lint.sh` fails if that
moves. Only `ui` draws; `json` and `prompt` reach for no external crate.

## The widgets

Nothing in `ui` is a screen. A widget is handed its strings and gives back what
was answered, so the text of every question stays with the command that asks it
and this crate holds the geometry alone. Everything draws into a bounded region
of the normal scroll rather than an alternate screen, which is what keeps output
readable when it is piped or redirected.

## Working on it

    ./lint.sh          the dependency floor, formatting and the tests
    ./lint.sh --fix    format

To work on this crate and a consumer together, put a `paths` override in the
consumer's `.cargo/config.toml`. It redirects a git dependency, leaves
`Cargo.lock` alone and passes `--locked`. A `[patch]` rewrites the lock instead.
An override must name the same package as the dependency it replaces, so it
cannot stand in across a rename of this package.

The widget tests draw into a `TestBackend` and assert on the rendered buffer,
against copies of the strings the installer and the tool pass in. A copy that
drifts from the real string is a test that guards nothing, so the fixtures are
checked against their originals whenever either moves.

## Licence

Apache 2.0. See [LICENSE](LICENSE).
