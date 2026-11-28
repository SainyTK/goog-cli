# Vision

`goog` exists so that everything a person does in Google Workspace can be done from a terminal, across all of their Google accounts at once.
It serves power users and AI agents who work in Drive, Docs, Sheets, Slides, Calendar and Gmail, and it turns one command into the result the browser would have given.
It owns exactly one thing: command-line access to those six apps for people signed in with their own Google accounts.

## Every operation, from the terminal

Coverage is measured against the browser: if a person can do it in the Drive, Docs, Sheets, Slides, Calendar or Gmail web app and Google's API allows it, `goog` should be able to do it.
Common work gets high-level commands that take human anchors such as text, names, folders and browser URLs, and hide UTF-16 indexes and raw request shapes.
Every service keeps a raw path (native batch updates and request bodies), so a missing high-level command never blocks the last mile of the API.
When Google's API cannot do something, `goog` says so in its help and README, and leaves it undone; it never falls back to browser automation or scraping.
Coverage grows by going deeper in the six supported apps.
The command surface stays small: one verb per job, grouped by service, and a new command that overlaps an existing one is folded into it.

## Many accounts, one login each

A person logs in to each Google account once, then works on any file, document, spreadsheet, presentation, event or message without switching accounts.
A command given a resource ID or URL finds the account that can open it: the remembered account for that resource, then the active account, then the other accounts in order, and it remembers the winner.
`--account` is strict: it uses exactly that account and never falls back.
Listing and search commands target no single resource, so they use the active account or `--account`.
Login asks for every supported scope on one consent screen; ordinary commands never open a browser, and a token missing a scope fails with the one command that fixes it.
Tokens live in one local file under the user's home directory, so agents run without operating-system password prompts, and `goog` treats that file as full account access.
Each user brings their own OAuth app; a shared, Google-verified app may come later only as an opt-in, with bring-your-own staying the default.
`goog` acts only as people signed in with their own Google accounts.

## People and agents at the same terminal

Human-readable output is the default, and `--json` gives the full structured result for scripts and agents.
Progress bars, update notices and other side output go to stderr and never change stdout, JSON validity or the exit status.
Writes are predictable: ambiguous targets are refused, `--dry-run` previews the exact change, and revision guards stop an edit from overwriting someone else's newer one.
Work in other people's files respects their ownership: comments anchor to text, and edits can be written as suggestions for the owner to accept or reject.
Errors name the account, the resource and the next command to run.
The agent skill in `skills/goog` documents the same commands and adds no behavior the CLI lacks.

## Verified against real Google

Every new or changed Google API behavior is checked against a live connected account before it merges, using throwaway `goog-e2e-` files that are trashed afterwards.
Unit tests use recorded HTTP fixtures and obvious placeholder IDs; real document IDs, URLs and email addresses are never committed.
API types are written by hand for the fields `goog` reads, and generated Google client crates are left out.
macOS, Linux and Windows are all supported targets, and releases ship only through GitHub Releases.

## Scope

`goog` ships as a command-line tool and an agent skill; MCP servers, TUIs, GUIs, background sync daemons and hosted services are out of scope.
Forms, Tasks, Keep, Chat, Meet, Contacts and other Workspace apps are out of scope for now.
Workspace administration (the Admin SDK, users, groups and domain settings) and service accounts with domain-wide delegation are out of scope.
`goog` runs no server, and tokens stay on the user's machine.
Release builds send usage counts (command name, result, duration, local hour and weekday, version and OS, with random device and hashed account IDs) to PostHog, show a notice the first time, and stop when the user opts out; arguments, resource IDs, contents and email addresses never leave the machine.
Other office suites and non-Google services are out of scope.

A change aligns when it lets a person do another real Workspace operation from the terminal in one of the six apps, makes working across several accounts need fewer flags or switches, makes output clearer for people or more reliable for agents, makes a write safer or easier to preview, or turns a raw-request workflow into a high-level command.
A change should be resisted when it adds a service outside the six, administers a domain or acts as anyone but a signed-in user, adds a surface beyond the CLI and the skill, sends tokens or files to any server, opens a browser or a prompt during an ordinary command, breaks stdout or JSON for agents, duplicates an existing command, imitates the browser where the API falls short, or lands Google API behavior that nobody verified against a live account.
