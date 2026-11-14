# Language guide

gdl is a small imperative language. A script runs top to bottom and every `obj`, `block` or trigger call adds one
object to the level.

## Syntax

```
// comments run to the end of the line
let x = 10                    // variables
x += 5                        // = += -= *= /=
let s = "a" + 1               // strings (+ joins), numbers, true / false / nil

if x > 3 && x < 100 { ... } else if x == 0 { ... } else { ... }
for i in 0..10 { ... }                // 0 up to 9
for i in 0..100 step 10 { ... }
for i in 10..0 step -1 { ... }
while x > 0 { x -= 1 }

fn spike_row(x, n) {                  // functions, recursion, return
  for i in 0..n { block("spike", x + i * 30, 15) }
}
```

- A new line ends an expression, so a line starting with `-` or `(` begins a new statement.
- Functions can read and assign top-level variables. `PI` is predefined.
- Top-level functions can be called before they are defined.
- Positions are in units: **30 units = 1 block**. `x = 15, y = 15` is the centre of the first block.

## Calling things: positional and named arguments

```
move(0, 45, 3, 0, 60, 2)                        // positional
move(0, 45, 3, 0, 60, 2, easing = EASE_IN_OUT)  // optional settings by name
move(x = 0, y = 45, target = 3, dx = 0, dy = 60, secs = 2)
```

Named arguments go after positional ones. They also work for your own functions
(`spike_row(n = 4, x = 100)`). A mistake gives a message that lists the valid names.

Every trigger also accepts `spawn_triggered`, `touch_triggered` and `multi_triggered` by name.

## Builtins

| call | does |
|---|---|
| `obj(id, x, y)` | create any object by id; returns its **handle** |
| `block(name, x, y)` | create a named block or object, e.g. `block("spike", 45, 15)` ([list](BLOCKS.md)) |
| `rect(id, x, y, w, h)` | fill a `w` by `h` grid, 30 units apart; returns the count |
| `set(handle, key, value)` | set any raw property key |
| `add_group(handle, g)` | put an object in group `g` |
| `group()` / `color()` | next unused group / colour channel |
| `set_rotation(h, deg)` `set_scale(h, s)` | rotate / scale an object |
| `flip_x(h)` `flip_y(h)` | flip an object |
| `set_z(h, layer, order)` | z layer and order |
| `set_color(h, ch)` `set_detail_color(h, ch)` | colour channels |
| `trigger(id, x, y)` | any trigger by object id (then `set()` its properties) |
| *45 trigger functions* | `move`, `pickup`, `item_edit`, `gradient`... see [TRIGGERS.md](TRIGGERS.md) |
| `print(...)` | log text (shown by `run` and the editor) |
| `floor ceil round abs sqrt sin cos` | one-argument math |
| `min max pow` | two-argument math |

`group()` and `color()` start after any group or colour already used by objects in a `.gdlrust` project, so
generated groups never collide with ones already in the level.

## Constants

| prefix | meaning | example |
|---|---|---|
| `B_` | block / object id from the catalog | `B_SPIKE` |
| `ID_` | trigger and gameplay object ids | `ID_FOLLOW` |
| `P_` | raw property keys for `set()` | `P_TARGET` |
| `EASE_` `ELASTIC_` `BOUNCE_` `EXP_` `SINE_` `BACK_` | easing curves | `EASE_IN_OUT` |
| `OP_` `CMP_` `ITEM_` `ROUND_` `SIGN_` | item edit and compare options | `OP_ADD`, `CMP_GTE` |
| `PICKUP_` `INSTANT_` `STOP_` | pickup, instant count, stop modes | `PICKUP_MULTIPLY` |
| `XY_` `EDGE_` | camera options | `EDGE_LEFT` |
| `BLEND_` `LAYER_` | gradient options | `LAYER_B1` |
| `CH_` | special colour channels | `CH_BG` |

All constants are listed in [TRIGGERS.md](TRIGGERS.md#constants) and [BLOCKS.md](BLOCKS.md).

## Any trigger, any property

For a trigger that isn't wrapped (area, shader, keyframe, advanced follow...), create it by id and set properties by key:

```
let t = trigger(ID_FOLLOW, 200, 15)
set(t, P_TARGET, 3)
set(t, P_CENTER, 4)
set(t, 72, 1)          // raw numbers work too
```

To discover the ids and keys of something, place it in the game editor, export the level, and run
`gdl objects level.gdlvl`.

## Limits

3,000,000 statements, 150,000 objects and 100 nested calls per run. They exist so a hosted editor can't be hung by a
bad script, and are enforced in the CLI too.

## Errors

Errors carry the line number (`line 4: ...`). In the web editor an error shows under the script and the last good
preview stays on screen.
