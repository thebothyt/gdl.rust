# The .gdlrust format

A `.gdlrust` file is one plain UTF-8 text file holding a whole project. It is meant to be read, edited, diffed and
committed like source code.

**What it contains:** your script, the objects that were already in the level, the level header, and the level's
XML wrapper (with the level string replaced by a placeholder).

**What it never contains:** cookies, browser storage, save-game data, account data you typed, or compiled binary.
The editor does not read or write any of those either. A project is exactly this file.

## Layout

```text
#gdlrust 1
name: My Level
created_with: gdl 0.1.0
wrap: base64
k4: compressed
header: kS38,1_40_2_125_3_255
%%% script
let g = group()
...
%%% objects
1,1,2,15,3,15,57,4
1,8,2,45,3,15
%%% template
<?xml version="1.0"?>
<plist ...><dict><k>k2</k><s>My Level</s><k>k4</k><s>{{LEVEL}}</s></dict></plist>
```

| part | meaning |
|---|---|
| `#gdlrust 1` | magic and format version. Files with a newer version are rejected with a clear message |
| `name` | project name (taken from the level's name when converting) |
| `created_with` | informational; ignored when reading |
| `wrap` | outer layer of the original `.gdlvl`: `xml`, `gzip`, `base64` or `base64-urlsafe` |
| `k4` | `compressed` if the level string was gzip + base64, `raw` if plain text |
| `header` | the level header (colours, settings), kept verbatim |
| `%%% script` | your gdl script |
| `%%% objects` | objects already in the level, one record per line, without the trailing `;` |
| `%%% template` | the level XML with the level string replaced by `{{LEVEL}}`. Optional |

Section markers are lines starting with `%%% `. They can never start a valid script line (`%` is an operator), so a
script cannot collide with them. Unknown header keys are ignored, so newer tools can add fields without breaking old
files.

## Converting and compiling

```text
.gdlvl  --convert-->  .gdlrust  --compile-->  .gdlvl
```

`gdl convert Level.gdlvl` decodes the level and stores its header, objects and wrapper. With the default empty script,
compiling again gives a level with the **same header and objects**. (The bytes can differ because the gzip stream is
re-encoded; the content does not.) This is covered by tests for every supported wrapping.

`gdl compile Project.gdlrust` runs the script and writes:

```text
header ; imported objects ; objects made by the script
```

back into the template, re-applying the original `wrap` and `k4` encoding. Imported objects are never altered or
reordered.

**Group and colour ids.** Before running the script, the tool scans the imported objects and starts `group()` and
`color()` above the highest group / colour id they use, so new groups don't collide with existing ones. (It checks
properties 57, 51, 71 and 21 to 23. Ids you pick by hand are yours to manage.)

**Fresh projects.** `gdl new Name` makes a project with no template, which cannot be compiled alone. Compile it into a
level with `gdl compile Name.gdlrust --into Level.gdlvl`, which appends the script's objects to that level.

## Safety

- Compiling and converting never modify the input. Output goes to a new file (`<name>.compiled.gdlvl`,
  `<name>.gdlrust`), and an existing file is never replaced without `--force`.
- Keep backups of levels anyway. The output has been checked structurally and by round-trip tests, but not
  exhaustively inside the game.
