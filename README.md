# Mistral usage in Omarchy Agents

Source: [WhiteHades/omarchy-mistral-usage](https://github.com/WhiteHades/omarchy-mistral-usage).

Mistral's Vibe Code and API budgets inside the existing Agents popup. This is a native `omarchy.agents` clone, created with `omarchy plugin clone omarchy.agents`, with a small Mistral data overlay. The project is the source of truth for plugin `efaz.agents`.

Each budget uses the same `CompactLimit` and `Meter` components as OpenCode and Codex: a label, meter and the native relative countdown, such as `22d 11h`. Hover a row for one line such as `$1.88 of $150.00 used`. Examples and test fixtures use synthetic usage figures. Both allowances come from Mistral, not fixed prices. There are no visible percentages, calendar dates, clock times, overage labels or explanatory footers for Mistral.

## Install and controls

Requires Omarchy, Rust/Cargo, mise, util-linux (`setpriv`), jq and mise-managed `pypi:mistral-vibe`. Sign in through Vibe first.

```sh
bin/install
omarchy shell omarchy.agents show
omarchy shell omarchy.agents hide
```

Click the usual Agents icon. Opening the popup refreshes limits; middle-clicking its icon runs the normal full refresh. Existing account controls, provider order, provider enable/disable settings, token statistics and other providers retain their native behavior. Mistral is controlled by the existing `vibe` provider setting.

The native clone metadata replaces the Agents entry rather than adding a second icon. These supported commands switch between the original and extended popup while preserving layout/settings:

```sh
omarchy plugin enable omarchy.agents  # stock Agents, Mistral extension off
omarchy plugin enable efaz.agents    # extended Agents, Mistral extension on
```

`bin/install` builds the helper, copies runtime files into `~/.config/omarchy/plugins/efaz.agents`, and enables the clone. It loads `/etc/omarchy.conf` if necessary and refuses unrelated target paths. The dotfiles tasks `omarchy:mistral:install` and `omarchy:mistral:check` own workstation deployment and verification. Omarchy may retain imported QML/JS source during updates; use `omarchy restart shell` while unlocked if the installed UI stays cached. Installation never restarts the shell automatically.

The old standalone `whitehades.mistral-usage` plugin was removed through Omarchy's plugin manager. Its native hidden backup is outside this project. There is no separate service, persistent helper, second bar entry or second writer of usage records.

## Data and authentication

The Rust helper invokes a small embedded Python bridge in Vibe's isolated interpreter. It reuses Vibe's `ApiKeyPlatformCredentialProvider` and `MistralPlatformHTTPClient`, including native keyring/OIDC handling and TLS verification. It reads only the official `https://api.mistral.ai/v1/billing/budget` and `billing/subscription` endpoints, without redirects. Credential identity is checked before and after the requests. No credentials or account fingerprints are returned or stored by this plugin.

Vibe's default `MISTRAL_API_KEY` credential precedence applies. An incompatible future Vibe auth API or an unsuccessful read produces unavailable data rather than 0% usage. Reports stay in memory. The stock JSON usage records, sync data and packaged files are not modified by the Mistral overlay.

Remaining money is allowance minus allowance multiplied by reported percentage divided by 100. Amounts are rounded for display and describe included credit, not an exact invoice or paid-overage ledger. A zero allowance is omitted.

Server reset timestamps take precedence. If absent or invalid, the expected boundary is the first of the next month at 00:00 UTC, anchored to the observation month. The UI uses the native days/hours/minutes countdown, never an absolute timestamp. It requests fresh data after an expired reset without zeroing usage locally.

The helper deadline is 15 seconds. Linux parent-death signaling stops its credential client when the helper is stopped. Collection follows the native Agents refresh cycle and stops with the clone. No model prompts or added transcript scans are involved.

## Native source and compatibility

`Main.qml`, `Panel.qml`, `Agent.qml` and the provider assets derive from Omarchy 4.5.0-3. `UPSTREAM.sha256` records the packaged baseline. The custom changes add `MistralUsage`, overlay its report on the `vibe` provider, and reuse the existing compact meter with an optional balance tooltip. Colors, fonts, geometry, controls, popup behavior and keyboard handling come from the native panel. No custom theme palette is maintained.

Like other Omarchy clones, these copied native files require review after upstream shell changes. Compare them with `/usr/share/omarchy/shell/plugins/agents/` before rebasing; do not overwrite the packaged source.

## Checks

```sh
omarchy plugin validate .
CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/omarchy-mistral-usage/build" cargo test --locked
cargo fmt --check
bin/omarchy-mistral-usage
omarchy shell omarchy.agents mistralStatus
```

Four focused Rust checks cover percentage units, money validation, unavailable/exhausted budgets and real QML money/UTC month-boundary formatting. `--validate` accepts a budget response on stdin for an offline check. Native IPC and screenshots verify the installed popup. Vibe 3.1.0 authenticated budget reads work. Current-theme appearance is checked; not every theme, locale or monitor configuration has been tested.

## Attribution and sources

Omarchy's native Agents source and assets are MIT licensed. Provider names and logos remain their owners' trademarks. This independent plugin is not an official Mistral product.

- [Mistral billing client](https://github.com/mistralai/mistral-vibe/blob/v3.1.0/vibe/app_server/_billing.py)
- [Mistral usage limits](https://docs.mistral.ai/admin/billing-usage/usage-limits)
- [Qt number formatting](https://doc.qt.io/qt-6/qml-qtqml-number.html)
- Installed Omarchy specification: `/usr/share/omarchy/shell/plugins/README.md`.
