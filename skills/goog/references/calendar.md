# Google Calendar operations

Use `goog calendar` for calendars, calendar-list settings, events, sharing rules, colors, and availability.
Use the installed `goog` binary outside this repository.
Inside the goog-cli repository, use `target/debug/goog` when it is current, or `cargo run --` when it is not built.

## Contents

- Preflight
- Calendars and calendar-list entries
- List and inspect events
- Create events
- Update, move, and delete events
- Sharing and colors
- Free-busy queries
- Completion gate

## Preflight

```bash
goog auth list
goog calendar --help
```

Use the active account unless the user specifies another authorized account.
Pass `--account EMAIL` when account routing must remain explicit.
Use `primary` only when the request targets the selected account's primary calendar.
Use explicit calendar IDs for shared or secondary calendars.
If a command reports missing scopes, run `goog auth login` once and retry it.

## Calendars and calendar-list entries

```bash
goog calendar calendars list --all --json
goog calendar calendars get CALENDAR_ID
goog calendar calendars create --summary "Project calendar" --time-zone Asia/Bangkok
goog calendar calendars patch CALENDAR_ID --summary "New title"
goog calendar calendars list-entry --help
```

Use `calendars update` only when replacing the editable metadata as a complete resource.
Use `calendars patch` for a partial metadata change.
Use `calendars delete` only for a secondary calendar after resolving its exact ID.
Use `calendars list-entry` to manage how a calendar appears for the authenticated user without deleting the underlying calendar.

## List and inspect events

```bash
goog calendar events list primary --time-min 2026-07-21T00:00:00+07:00 --time-max 2026-07-22T00:00:00+07:00 --single-events --order-by start-time --json
goog calendar events get CALENDAR_ID EVENT_ID
goog calendar events instances CALENDAR_ID RECURRING_EVENT_ID --json
```

Use explicit RFC3339 bounds for predictable event windows.
Pass `--single-events` before `--order-by start-time` when recurring events must be expanded chronologically.
Use `--all` only when every result page is required.
Use `--sync-token` only with a token returned by a compatible earlier full list operation.

## Create events

```bash
goog calendar events create primary --summary "Planning review" --start 2026-07-22T10:00:00+07:00 --end 2026-07-22T11:00:00+07:00 --time-zone Asia/Bangkok
goog calendar events create primary --summary "Company holiday" --start 2026-07-23 --end 2026-07-24 --all-day
goog calendar events quick-add primary "Lunch with Sam tomorrow at noon"
```

Repeat `--attendee EMAIL` for multiple attendees.
Use `--google-meet` to create a Meet conference.
Repeat `--recurrence` for recurrence entries and `--reminder METHOD:MINUTES` for reminder overrides.
Use `--no-reminders` only when default reminders must be disabled.
Set `--send-updates all`, `external-only`, or `none` deliberately when guests are affected.
Use `--event PATH` or `--event -` when the full Event resource is required.

For all-day events, Google Calendar uses an exclusive end date.
An event covering July 23 uses `--start 2026-07-23 --end 2026-07-24 --all-day`.

## Update, move, and delete events

```bash
goog calendar events patch CALENDAR_ID EVENT_ID --summary "Updated planning review"
goog calendar events move SOURCE_CALENDAR_ID EVENT_ID --destination DESTINATION_CALENDAR_ID
goog calendar events delete CALENDAR_ID EVENT_ID --send-updates all
```

Use `events patch` for partial updates.
Use `events update` only when replacing the event as a complete resource.
Resolve both calendar ID and event ID before mutation.
Choose `--send-updates` deliberately for changes or deletions involving attendees.
Read the event back after creation, update, import, or move.
Confirm absence with a bounded list after deletion.

## Sharing and colors

```bash
goog calendar acl list CALENDAR_ID --json
goog calendar acl add CALENDAR_ID --scope user --value person@example.com --role reader --json
goog calendar acl patch CALENDAR_ID RULE_ID --role writer --json
goog calendar colors get --json
```

ACL roles are `none`, `free-busy-reader`, `reader`, `writer`, and `owner`.
ACL scopes are `default`, `user`, `group`, and `domain`.
Omit `--value` only for the default scope.
Resolve the exact rule ID before patching, replacing, or deleting a rule.
Use a color ID returned by `goog calendar colors get` rather than guessing one.

## Free-busy queries

```bash
goog calendar freebusy --time-min 2026-07-22T09:00:00+07:00 --time-max 2026-07-22T17:00:00+07:00 --calendar primary --calendar team@example.com --time-zone Asia/Bangkok --json
```

Repeat `--calendar` for each calendar or group.
Use explicit RFC3339 bounds and the desired response time zone.

## Completion gate

- The operation used the intended account and exact calendar ID.
- Event times, dates, time zones, recurrence, reminders, attendees, and notification behavior match the request.
- Mutated events or calendar metadata were read back through `goog`.
- Sharing changes were verified by listing or reading the resulting ACL rule.
- Destructive operations targeted an exact resource ID and were verified.
- The final response includes useful calendar and event IDs or native URLs returned by the CLI.
