# Code signing policy

**Status:** the Windows installers are not signed yet. The project is applying for free code signing provided by [SignPath.io](https://signpath.io), with a certificate by [SignPath Foundation](https://signpath.org). This page is the policy that applies from the first signed release, and it will say so here when that happens.

What an installed copy updates itself from is already signed, with the project's own update key (see [development.md](development.md)). That is a different thing from the code signing described here: it protects updates, and Windows and macOS do not look at it.

## What is signed

Only the installers and programs built from the source code in this repository, [YashIngole/agent-moshpit](https://github.com/YashIngole/agent-moshpit), by its release workflow ([.github/workflows/release.yml](../.github/workflows/release.yml)) on GitHub's own runners, from a version tag. Nothing built on anyone's own computer is signed, and nothing from another project is signed under this project's name.

Each release needs a person's approval before it is signed, and again before it is published.

## Team roles and their members

- **Committers and reviewers:** [YashIngole](https://github.com/YashIngole)
- **Approvers:** [YashIngole](https://github.com/YashIngole)

A change proposed by anyone else (a pull request) is reviewed by a committer before it is merged. Members use multi-factor authentication for GitHub and for SignPath.

## Privacy

Agent Moshpit has no telemetry and no analytics, and keeps no account.

It makes two requests of its own, both about versions, when it starts and every 12 hours: it asks npm for the newest version of each agent program you have installed, and it asks this repository's releases on GitHub for a newer version of itself. Setting `MOSHPIT_NO_UPDATE_CHECK` turns both off. An update is fetched only when you choose it in the menu.

The agent programs it starts (Claude Code, Codex and the rest) are other people's software, started at your request with your own sign-ins. They talk to their own services as they do in any terminal, under their own privacy policies.

The website, agentmoshpit.com, sets no cookies and counts visits with Cloudflare Web Analytics.
