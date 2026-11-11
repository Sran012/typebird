# Security Policy

## Supported versions

Only the latest release is supported. Older versions do not receive security fixes.

## Reporting a vulnerability

Please report privately using GitHub's private security advisory feature:

https://github.com/Sran012/typebird/security/advisories/new

Do not open a public issue for a vulnerability. I will acknowledge the report within 72 hours and work on a fix. Once a fix is released, the advisory is published and you are credited unless you prefer to stay anonymous.

## Scope

In scope:

- arbitrary code execution or privilege escalation through typebird or the installer
- path traversal or unsafe file handling in `install.sh` or `uninstall.sh`
- tarball or model integrity bypasses
- unintended network access at runtime
- focus or keyboard handling that leaks keystrokes to the wrong window
- anything that lets a non-authorized user read input or synthesize keys

Out of scope:

- the intended ability to read input devices and synthesize keys through uinput. This is documented in the README and is the core of the tool.
- accuracy or hallucination behavior of whisper.cpp. Report those upstream.
- issues in third-party dependencies. Report those upstream; I will update the dependency.

## The input access capability

typebird needs read access to `/dev/input/event*` and write access to `/dev/uinput`. Together that is the ability to read every keystroke and synthesize arbitrary ones. This is inherent to a global dictation shortcut. The installer adds you to the `input` group and installs a udev rule. If that is not acceptable for your threat model, do not install the hotkey: bind a desktop shortcut to `typebird toggle` instead.
