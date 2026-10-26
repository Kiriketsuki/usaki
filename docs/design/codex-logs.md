# Codex session logs

The `CodexLogsProvider` in `crates/usaki-core` reads the Codex usage from the session logs.
It never reads `auth.json`, never calls an OpenAI endpoint and never starts `codex app-server`.

## Path rule

```
${CODEX_HOME:-$HOME/.codex}/sessions/YYYY/MM/DD/rollout-*.jsonl
```

- An empty `CODEX_HOME` counts as unset.
- On Windows, `USERPROFILE` replaces `HOME` when `HOME` is unset.
- Older sessions are `.jsonl.zst` files. The provider skips them.
- A Codex home without a `sessions` folder gives no account.

## Event shape

Codex writes one `token_count` event to the log of each running session. The provider reads
only the `rate_limits` object.

```json
{
  "timestamp": "2027-01-15T07:43:20.000Z",
  "type": "event_msg",
  "payload": {
    "type": "token_count",
    "rate_limits": {
      "limit_id": "codex",
      "primary": { "used_percent": 37.0, "window_minutes": 10080, "resets_at": 1800300000 },
      "secondary": null,
      "credits": { "has_credits": true, "unlimited": false, "balance": "123.4500" },
      "plan_type": "pro"
    }
  }
}
```

## Read rule

1. The provider lists every `.jsonl` file under `sessions` and sorts the files by their
   modified time, newest first. A resumed session appends to its old file, so the folder date
   is not the age of the data.
2. It reads each file from the end and takes the last `token_count` line that passes the
   checks below. It stops when the next file is older than the best event so far.
3. It keeps the last event of each file in a cache with the size and the modified time of the
   file. A poll does not read a file again until one of the two changes.

An event passes when all of these are true:

- `type` is `event_msg`, and `payload.type` is `token_count`.
- `rate_limits` is an object, not `null`.
- `limit_id` is `codex`. A missing `limit_id` also counts as `codex`. The provider skips any
  other id, such as a model limit.
- At least one of `primary` and `secondary` holds a window. A window has a `used_percent`
  that is not negative and a `window_minutes` over 0.
- No two windows have the same `window_minutes`.

The provider maps each window by `window_minutes`: 300 is the 5h window and 10080 is the 7d
window. It never maps by `primary` or `secondary`. On the Pro plan Codex reports only the 7d
window. Other plans can report both windows, in either slot.

The time of the event is its `timestamp`. A line without one uses the modified time of the file.

## Privacy

The session logs hold whole conversations. The parser skips each line that does not hold the
text `token_count`. Its typed structs name only the fields above, so no conversation text
stays in memory. No log line and no parse error leaves the provider, because a serde error
can quote the line.

## Staleness rule

The logs update only while Codex runs.

- A window whose `resets_at` has passed has reset. It reports 0% used and `ready`.
- The account is stale when the newest event is more than 6 hours old **and** no window is
  still open. A window is open when its `resets_at` is in the future.
- An open window keeps its percent. A usage after the event can only raise it, so the shown
  percent is a lower bound.
- No event and no `sessions` folder both give a stale account with no windows and a reason.

## Account

The default Codex home is one account named `codex`. Its wire id is `codex:codex`. The name
does not follow the plan type, because a plan change then drops a pin. The config file in
milestone 3 adds more Codex homes, each with its own name.

## Credits

The card shows the `credits.balance` string as Codex writes it, under the label `credits`. The
logs do not name the unit, so the card adds none. An `unlimited` account shows `unlimited`.
