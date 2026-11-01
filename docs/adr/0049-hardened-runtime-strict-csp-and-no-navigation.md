# 0049. Hardened runtime, strict CSP and no navigation

## Status

Accepted (2026-11-01).

## Context

Taskboard renders content it doesn't control: Jira's HTML from many authors, inside a webview that can call the
app's commands, and it holds an API token in the Keychain. The build was signed without the hardened runtime, the
CSP allowed inline `<style>` elements, and nothing stopped a link or script from loading another page in the app's
webview.

## Decision

- **Hardened runtime.** `scripts/sign-app.sh` signs every build with `--options runtime`: no injected libraries,
  no debugger attach. The app needs no entitlements: it isn't sandboxed, and neither the Keychain nor
  notifications need one; WebKit runs its scripts in its own processes.
- **Stricter CSP.** Only the bundled style sheets load (`style-src-elem 'self'`); `style` attributes, which React
  sets, stay allowed. `base-uri` and `form-action` are `'none'`.
- **No navigation.** A plugin on every webview (`window/navigation.rs`) allows only the app's own pages; anything
  else is blocked and logged. Web links already open in the browser.

## Consequences

- Inline `<style>` elements, from a library or pasted content, are blocked; styles live in `src/styles/`.
- The signing certificate is still local (ADR 0015), so there is no notarization.
