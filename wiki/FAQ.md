# FAQ

**Is VoidGuard a remote-control / C2 tool?**
No. It's a local monitoring and administration tool for a machine
you're sitting at (or otherwise authorized to administer). It has no
remote-control, shell-execution, or covert-persistence capability —
see [[Security-Model]].

**Can it run arbitrary commands I give it?**
No, by design. Every action is a specific, typed operation (list
processes, terminate a PID, open/close a specific port, etc.) — there
is no "run this command" feature and there will not be one.

**Does the website let me control my PC remotely?**
No. The GitHub Pages site is documentation and a project overview
only. VoidGuard itself is a desktop app; nothing about it is
reachable from the web site.

**Will it flag my unsigned/uncommon software as malware?**
It reports evidence (unsigned, unusual location, newly added to
startup, etc.) and a confidence level — it does not label anything
"malware" outright, and it never auto-deletes or auto-quarantines.

**What platforms does it support?**
Windows 10 and 11. It's built with Tauri + Rust and uses Windows-only
APIs for anything privileged, so it won't run as a monitoring tool on
other OSes (though the frontend can be developed there).

**How is data stored?**
Locally, in a SQLite database at `%APPDATA%/VoidGuard/voidguard.db`.
Nothing is sent anywhere.

**I found a way it could be used maliciously — what do I do?**
Open an issue with the **Security concern** template, or see
`CONTRIBUTING.md` for how to report something privately if it's a
live, exploitable issue in a released version.
