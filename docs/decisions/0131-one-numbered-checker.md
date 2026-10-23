# 0131. One numbered checker, in blue

- **Status:** Accepted
- **Date:** 2026-10-23
- **Replaces part of:** [0107](0107-numbered-checker.md) (the checker's colours following the accent)

## Problem

The numbered checker took its colours from the accent colour. The project lead found the blue accent's checker the clearest and kept switching to the blue accent just to see it, though he prefers the standard red for the rest of the app.

## Decision

The numbered checker always uses the blue accent's pair of colours (blue at A1, through teal, to green at H8), whatever the accent. The accent setting no longer changes it.

## Consequences

- The checker looks the same for everyone, so screenshots and notes about cell colours match.
- The image is made once; switching accents no longer rebuilds it.
