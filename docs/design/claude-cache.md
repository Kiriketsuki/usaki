# Claude cache file contract

The statusline writes one cache file per Claude account. The `ClaudeCacheProvider` in
`crates/usaki-core` reads it. The statusline writer is a later task.

## Path rule

```
${XDG_CACHE_HOME:-$HOME/.cache}/usaki/claude-<acct>.json
```

- An empty `XDG_CACHE_HOME` counts as unset.
- On Windows, `USERPROFILE` replaces `HOME` when `HOME` is unset.
- `<acct>` is the account id, for example `acc1`. It may hold only `A-Z`, `a-z`, `0-9`, `_` and `-`.
  The reader rejects any other id with an error, unless the account sets an explicit source path.
- An account can set an explicit source path. That path replaces the rule above.

The writer must write a temp file and then rename it. This keeps a half-written file from reaching the reader.

## Flat shape

The statusline reads these Claude Code stdin fields: `rate_limits.five_hour.used_percentage`,
`rate_limits.five_hour.resets_at` and the same two for `seven_day`.

```json
{
  "account": "acc1",
  "five_hour": { "used_percentage": 42, "resets_at": 1800003600 },
  "seven_day": { "used_percentage": 17, "resets_at": 1800300000 },
  "written_at": 1800000000
}
```

## limits[] shape

The parser also accepts the newer shape.

```json
{
  "account": "acc2",
  "limits": [
    { "window_minutes": 300, "used_percentage": 61, "resets_at": 1800003600 },
    { "window_minutes": 10080, "used_percentage": 33, "resets_at": 1800300000 }
  ],
  "written_at": 1800000000
}
```

When `limits` is an array, the parser uses it and ignores `five_hour` and `seven_day`.

## Field rules

| Field | Rule |
|---|---|
| `used_percentage` | A JSON number, int or float. Required inside a window. A negative value is an error. The parser keeps a value over 100 as it is, so the UI can show an over-limit state. |
| `resets_at` | Epoch seconds (int or float) or an RFC 3339 string. Optional. |
| `written_at` | Same formats as `resets_at`. Optional, but see staleness. |
| `window_minutes` | A whole number. 300 is the 5h window. 10080 is the 7d window. The parser keeps any other value as an "other" window. Two entries with the same value are an error. |
| `five_hour`, `seven_day` | May be missing or `null`. The snapshot then has no such window. |
| `account` | Informational. The reader uses the account id from its own config. |

## Staleness rule

- A snapshot is stale when `now - written_at` is more than the threshold. The default is 15 minutes.
- A snapshot is stale when `written_at` is missing.
- A `written_at` ahead of `now` by up to 2 minutes counts as clock skew. The age is then negative, so the snapshot is fresh, and `fetched_at` is `now`.
- A `written_at` ahead of `now` by more than 2 minutes is not trusted (a bad clock, or a writer that used milliseconds). The snapshot is stale, and `fetched_at` is `now`.
- A missing cache file gives a stale snapshot with no windows. It is not an error.
- A window whose `resets_at` is in the past has reset. It reports 0% used, and the snapshot stays.

## Errors

- A file that is not valid JSON gives an error that names the file and the parse problem.
- A field with the wrong type gives an error that names the file and the field.
- Any other read failure (for example, no permission) gives an I/O error.
