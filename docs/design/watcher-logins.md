# Watcher logins

The statusline cache file updates only while a Claude Code session runs in the account. A
watcher login lets usaKi show an account that has no running session.

## Design

A watcher login is a normal Claude Code login in a config dir that only usaKi uses:

```
<data dir>/usaki/watch/<acct>
```

- On Linux the data dir is `$XDG_DATA_HOME`, else `~/.local/share`. On Windows it is `%APPDATA%`.
- `<acct>` follows the account id rule of the cache file: only `A-Z`, `a-z`, `0-9`, `_` and `-`.
  usaKi skips a dir with any other name.
- No working session uses this config dir, so no working session shares its token chain. A
  refresh in the watcher dir cannot log out a working session.

The provider is `usaki_core::providers::claude_watch`. For each watcher dir it:

1. Reads `claudeAiOauth.accessToken` and `claudeAiOauth.expiresAt` (epoch ms) from
   `.credentials.json`. It opens the file for reading only.
2. Calls `GET https://api.anthropic.com/api/oauth/usage` with the headers
   `Authorization: Bearer <token>`, `anthropic-beta: oauth-2025-04-20` and
   `User-Agent: claude-code/<version>`. The version comes from `claude --version`. usaKi reads
   it again each hour, so a claude update shows up in the header.
3. Reads `five_hour` and `seven_day` from the reply. `utilization` is a percent from 0 to 100.
   `resets_at` is an RFC 3339 string. The provider ignores every other key.

## One-time login

Run this once for each account. Use the account id as the dir name.

```sh
CLAUDE_CONFIG_DIR="$HOME/.local/share/usaki/watch/acc1" claude auth login
```

usaKi finds the new dir on its next watcher tick (30 s). No restart is necessary.

## Source merge

The poller merges the two sources by account id. An account in both sources appears once.

1. A fresh statusline cache file wins. The watcher makes no call for that account.
2. Else the watcher reading, if it has one.
3. Else the cache result, stale or failed.
4. Else the watcher record with its stale reason.

## Rate rules

- At most one usage call per account per 5 minutes. The watcher has its own schedule. It does
  not follow the 30 s poll tick.
- On HTTP 429 the watcher doubles the interval, up to 60 minutes. A good reply resets it to 5 minutes.
- On HTTP 401 or an expired token the account is stale, with the reason.
- A 429 or a network failure keeps a reading fresh for 15 minutes. After that the account is stale.

## Refresh rule

usaKi never refreshes a token and never writes a credentials file. The official `claude` binary
does the refresh, in the watcher dir only.

1. When the token expires within 2 minutes, usaKi runs `claude auth status --json` with
   `CLAUDE_CONFIG_DIR` set to the watcher dir. The timeout is 120 s.
2. When the expiry did not change, usaKi runs `claude -p "/usage"` the same way. The timeout is 180 s.
3. When the first command runs past its timeout, usaKi does not start the second one. Two runs
   at the same time can spend one refresh token twice.
4. usaKi reads the credentials file again after each command.

usaKi finds the binary on `PATH`, then at `~/.local/bin/claude`. At a timeout usaKi sends
SIGTERM and lets a thread wait for the exit. It never sends SIGKILL, because a kill between the
use of the old refresh token and the write of the new one ends the login. A refresh uses the call
slot of the account, so the binary runs at most once per 5 minutes per account.

A busy flag guards each account. The flag is set while a refresh child of the watcher dir has
not exited. A child that ignores SIGTERM keeps the flag set for as long as it runs. While the
flag is set, usaKi starts no refresh and makes no usage call for that account, even when a call
slot is due. The thread that reaps the child clears the flag. The next tick then polls the
account again.
