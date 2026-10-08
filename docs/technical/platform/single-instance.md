# Single-Instance Protocol

Ensures only one Ferrite **process** runs at a time unless the user opts out. When a second instance is launched (e.g. “Open with Ferrite” on several files in Explorer), it forwards canonicalized paths to the already-running process over local TCP, waits for an `OK` handshake, then exits. The primary opens received paths in the **last-focused document window**.

Opt-out: Settings → Files → **Allow multiple Ferrite windows/instances** (`allow_multiple_instances`, default off), or CLI `--new-instance`. Either skip skips forwarding and writes **no** lock / pid / port files (`instance_listener = None`).

Multi-window file routing: [multi-window-file-routing.md](./multi-window-file-routing.md). macOS Finder Open With (separate from this IPC): [macos-open-with.md](./macos-open-with.md).

## Key Files

| File | Purpose |
|------|---------|
| `src/single_instance.rs` | OS lock, port publish, TCP handshake, accept thread, path canonicalization |
| `src/main.rs` | `load_config()` **before** `try_acquire_instance`; `--new-instance`; `InstanceAcquire` → listener or exit |
| `src/config/settings.rs` | `allow_multiple_instances: bool` (serde default `false`) |
| `src/ui/settings/files.rs` / `registry.rs` | Files entry `files.allow_multiple_instances` |
| `src/app/mod.rs` | Optional `SingleInstanceListener`; `set_instance_listener` |
| `src/app/file_ops.rs` | `handle_instance_paths()` — drain `InstanceIncoming`, open in focused window |
| `src/path_utils.rs` | `normalize_path` strips Windows `\\?\` after canonicalize |

## Startup

```text
Parse CLI
  → load_config()          (no logging/icons yet)
  → skip if --new-instance or settings.allow_multiple_instances
       → Independent (no files written, listener = None)
  else try_acquire_instance:
       File::try_lock(instance.lock)
         Ok        → bind 127.0.0.1:0, write port into lock + instance.port, write pid, become Primary
         WouldBlock → read port (lock file, else sidecar); retry ~2 s / 50 ms if missing
                      connect, send paths or __FOCUS__, wait 1 s for OK\n
                        OK     → Forwarded (exit)
                        no OK  → remove stale files, become Primary
```

`try_acquire_instance` returns `InstanceAcquire::{Primary, Forwarded, Independent}`.

## Lock and port publish

1. **`instance.lock`** — opened read/write/create; exclusive `std::fs::File::try_lock()` (stable 1.89; MSRV 1.92). The primary **holds the `File` handle** for the process lifetime (`SingleInstanceListener._lock_file`). After bind, the port is written into this file.
2. **`instance.port`** — same port as a readable sidecar. On Windows an exclusive lock is **mandatory**: a second handle cannot `read_to_string` the locked file (os error 33). Unix `flock` is advisory, so the lock file itself is readable; the sidecar is still written for a uniform read path.
3. **`instance.pid`** — primary PID for Windows `AllowSetForegroundWindow`.

Locations: Windows `%APPDATA%\ferrite\`, Linux `~/.config/ferrite/`, macOS `~/Library/Application Support/ferrite/`.

Drop of `SingleInstanceListener` releases the lock first, then deletes lock / port / pid files.

## Handshake

- Secondary sends UTF-8 paths (one per line) or `__FOCUS__` if none, `shutdown(Write)`, then reads `OK\n` with a **1 s** timeout.
- Primary `read_message_from_stream` parses the message, then writes `OK\n`.
- Missing `OK` (stale lock pointing at a non-Ferrite listener) → treat as no instance, remove stale files, become primary.

If the port is not yet written (winner still binding), the secondary retries connect for **~2 s** with **50 ms** sleeps before becoming primary.

## Path canonicalization

Each forwarded path is resolved in the **secondary** (not the primary’s CWD):

```text
canonicalize(p).or_else(|_| current_dir().map(|d| d.join(p)))
  → path_utils::normalize_path   // strip \\?\
```

Helper: `canonicalize_forward_path` in `src/single_instance.rs`.

## Primary accept loop

Unchanged delivery model: blocking `single-instance-accept` thread → `mpsc` → `ctx.request_repaint()` → UI `poll()` → `open_file_smart_in_window` on the last-focused window + `focus_document_window`.

## Edge cases

| Scenario | Behavior |
|----------|----------|
| Setting ON or `--new-instance` | No lock/pid/port; each process is independent |
| N Explorer “Open with” at once | One process wins `try_lock`; others forward after port publish |
| Stale lock, dead process | OS releases lock; new process acquires and overwrites port |
| Bogus port / non-Ferrite listener | Connect may succeed; no `OK` → become primary |
| Relative CLI path (`.\notes.md`) | Canonicalized against secondary CWD before send |
| Config dir unavailable | Warning; run as primary without lock |
| Listener bind failure | App runs; no IPC |

## Tests

In `src/single_instance.rs`: exclusive `try_lock` blocks a second handle and succeeds after drop; `read_message_from_stream` + `OK` round-trip; `./x.md` resolves against CWD and verbatim prefixes are stripped.

## Manual QA (Windows)

1. Select 5 `.md` files in Explorer → Enter → **one** window with 5 tabs.
2. Setting ON → each open spawns its own window.
3. `ferrite --new-instance` always spawns.
4. With Ferrite running: `ferrite .\relative.md` from a terminal opens the file.
5. Write a bogus port into `instance.lock` pointing at a PowerShell `TcpListener` → Ferrite still opens as a new primary.
