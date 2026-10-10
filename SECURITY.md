# Security policy

## Reporting a vulnerability

Please report security problems **privately**, not in a public issue:

1. Go to the repository's **Security** tab.
2. Choose **Report a vulnerability**.
3. Describe the problem, how to reproduce it, and what an attacker could do with it.

Aventail 3D is a hobby project, so responses are best effort. You can expect an acknowledgement within a week, and a fix or a plan as soon as practical. Reporters are credited in the release notes unless they prefer not to be.

## Supported versions

Only the latest release and the `main` branch receive security fixes.

## What counts as a vulnerability

The app opens files made by other people (`.rhy` projects, OBJ and glTF models), so these are in scope:

- A file that crashes the app, hangs it, or makes it use unbounded memory
- A file that makes the app read or write outside where it should
- Personal data leaking into saved projects, exports or logs (usernames, local paths)
- Weaknesses in how releases are built, signed or downloaded

Out of scope: bugs in Substance Painter, Godot or other programs that open the app's exports.

## How the app defends itself

See the Security section of [ARCHITECTURE.md](ARCHITECTURE.md): safe Rust, size limits and fuzz tests on every file reader, no telemetry, and automatic dependency audits on every change.
