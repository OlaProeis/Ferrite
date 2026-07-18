# Relative `.lnk` and `.url` shortcuts

This directory contains a PowerShell utility for creating folder shortcuts
whose target is stored relative to the shortcut's directory.

Files:

- `Relative_shortcut_url_lnk.ps1` — main script.
- `Relative_shortcut_url_lnk.bat` — batch wrapper.
- `RelativeShortcutLinkFactory.cs` — Windows Shell Link helper used by the
  PowerShell script.

## Basic usage

Running the batch file without arguments uses the editable defaults near the
top of the PowerShell script:

```bat
Relative_shortcut_url_lnk.bat
```

Example with explicit paths:

```bat
Relative_shortcut_url_lnk.bat ^
  -TargetPath "E:\Example\Perf\Fund" ^
  -OutputDirectory "E:\Example\Info\Fund" ^
  -Name "Perf" ^
  -Format Both
```

`Format` accepts `Both`, `Lnk`, or `Url`. If no name is supplied, the target
folder name is used. Existing output files are preserved unless `-Force` is
specified.

Relative `TargetPath` values are interpreted from `OutputDirectory`. A
relative shortcut cannot cross a drive or UNC-share boundary.

## How the PowerShell script loads the C# helper

The PowerShell script does **not** dot-source
`RelativeShortcutLinkFactory.cs`. Dot-sourcing applies to PowerShell scripts,
for example:

```powershell
. .\AnotherScript.ps1
```

PowerShell cannot dot-source a `.cs` file. Instead, the main script locates
the C# source beside itself and compiles it into the current PowerShell
process:

```powershell
$helperPath = Join-Path $PSScriptRoot 'RelativeShortcutLinkFactory.cs'

if (-not (Test-Path -LiteralPath $helperPath -PathType Leaf)) {
    throw "Required helper is missing: $helperPath"
}

if (-not ('RelativeShortcutSupport.LinkFactory' -as [type])) {
    Add-Type -Path $helperPath
}
```

`$PSScriptRoot` is the directory containing
`Relative_shortcut_url_lnk.ps1`. Therefore the `.cs` file must remain beside
the `.ps1` file.

`Add-Type -Path` performs four operations:

1. Reads `RelativeShortcutLinkFactory.cs`.
2. Compiles the C# source in memory.
3. Loads the resulting .NET assembly into the current PowerShell process.
4. Makes the public C# type available as
   `[RelativeShortcutSupport.LinkFactory]`.

The type check prevents PowerShell from trying to compile the same class a
second time if it is already loaded in the current process.

The PowerShell script then calls public static C# methods with standard .NET
syntax:

```powershell
[RelativeShortcutSupport.LinkFactory]::GetRelativePath(...)
[RelativeShortcutSupport.LinkFactory]::Create(...)
[RelativeShortcutSupport.LinkFactory]::Inspect(...)
```

No permanent `.exe` or `.dll` is created. The compiled helper exists only in
that PowerShell process. In short: the helper is **compiled and loaded at
runtime**, not dot-sourced.

## Why a relative `.url` might not work

A `.url` file is an **Internet Shortcut**, not a normal filesystem shortcut.
The generated file is intentionally simple:

```ini
[InternetShortcut]
URL=file:../../Perf/Fund
```

The path is a relative file URI. A relative URI needs an absolute base before
it can be resolved. Windows does not document a rule saying that the base must
be the directory containing the `.url` file.

The program that opens the `.url` chooses the base:

- Total Commander can treat the value as a filesystem path and use the
  shortcut's directory as its working directory. The shortcut then appears to
  work.
- Explorer passes the URL to the registered Internet Shortcut or browser
  handler. That handler can use its own working directory, search for the
  string, or reject it. The same `.url` can therefore fail in Explorer.

Forward slashes and URI escaping are still required. For example, spaces are
encoded as `%20` and a literal `#` is encoded as `%23`. Correct encoding does
not solve the missing-base problem.

An absolute `file:///E:/...` URL is dependable but is no longer relative or
portable. Use `.lnk` when reliable relative filesystem navigation is required.

## How the relative `.lnk` works

A `.lnk` is a binary Shell Link container. It can hold several independent
ways to locate its target:

1. An absolute item ID list (PIDL).
2. Absolute volume and path information (`LinkInfo`).
3. Distributed link-tracking information.
4. A `RELATIVE_PATH` StringData field.

Normal Windows shortcuts prefer the absolute and tracking information. The
generator deliberately creates a unique, nonexistent dummy target such as:

```text
C:\__RSL_45c5...__\d\Perf\Fund
```

The dummy directory is not created. It exists only inside the `.lnk` as the
primary target required by the Shell Link creation API.

The generator also stores the real relationship separately:

```text
..\..\Perf\Fund
```

It sets these Shell Link flags:

```text
HasRelativePath
IsUnicode
ForceNoLinkInfo
ForceNoLinkTrack
```

Windows resolves the generated link as follows:

```text
Try dummy absolute PIDL
        |
        v
Dummy target does not exist
        |
        v
LinkInfo and distributed tracking are disabled
        |
        v
Read RELATIVE_PATH from the .lnk binary
        |
        v
Combine it with the actual directory containing the .lnk
        |
        v
Open the resulting folder
```

For example:

```text
Shortcut directory:
E:\Example\Info\Fund

Stored RELATIVE_PATH:
..\..\Perf\Fund

Resolved target:
E:\Example\Perf\Fund
```

The generated `.lnk` is marked read-only. Without that protection, Explorer
can save the successfully resolved absolute target back into the shortcut,
silently destroying its persistent relative behavior.

## What Explorer Properties and Total Commander show

Explorer Properties and Total Commander's link-information viewer do not show
every structure stored inside a `.lnk`.

| Viewer | Usually shows | Does not normally show |
| --- | --- | --- |
| Explorer Properties | Primary target, arguments, working directory, icon | `RELATIVE_PATH`, resolution flags, fallback order |
| Total Commander Lister/linkinfo | Primary target returned from the absolute PIDL | `RELATIVE_PATH` StringData and the flags controlling resolution |
| `LinkFactory.Inspect` | Binary flags and the exact stored relative path | Nothing relevant is intentionally hidden |

Therefore a generated shortcut can display this misleading target:

```text
C:\__RSL_...__\d\Perf\Fund
```

while Windows actually opens the correct relative target. The viewer is
showing the primary dummy target, not the separate relative-path field used
after that dummy target fails.

Do not remove the read-only attribute and save changes through the Properties
dialog unless converting the shortcut back to an ordinary absolute shortcut
is intentional.

## Inspecting the hidden relative field

The included helper can read the binary field directly:

```powershell
$helper = Join-Path $PWD 'RelativeShortcutLinkFactory.cs'
if (-not ('RelativeShortcutSupport.LinkFactory' -as [type])) {
    Add-Type -Path $helper
}

[RelativeShortcutSupport.LinkFactory]::Inspect('E:\Path\To\Shortcut.lnk')
```

Important properties in the result are:

```text
RelativePath
HasRelativePath
HasLinkInfo
ForceNoLinkInfo
ForceNoLinkTrack
```

## References

- [Microsoft: Internet Shortcuts](https://learn.microsoft.com/en-us/windows/win32/lwef/internet-shortcuts)
- [Microsoft: Combining base and relative URLs](https://learn.microsoft.com/en-us/windows/win32/wininet/handling-uniform-resource-locators#combining-base-and-relative-urls)
- [Microsoft Shell Link specification: StringData](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/17b69472-0f34-4bcf-b290-eccdb8de224b)
- [Microsoft Shell Link specification: LinkFlags](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/ae350202-3ba9-4790-9e9e-98935f4ee5af)
- [Microsoft: How relative Shell Links are resolved](https://devblogs.microsoft.com/oldnewthing/20171019-00/?p=97247)
