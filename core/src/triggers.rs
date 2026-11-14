//! Table-driven Geometry Dash 2.2 trigger builtins.
//!
//! Every trigger is one entry in [`TRIGGERS`]: its script name, object ID and arguments.
//! The interpreter turns a call such as `pickup(300, 15, 1, 5, mode = PICKUP_MULTIPLY)` into
//! an object record from this table, and `gdl docs` generates `docs/TRIGGERS.md` from it, so
//! the table is the single source of truth for both behaviour and documentation.
//!
//! Property keys, object IDs and enum values were cross-checked against two independent
//! community sources: the `gdlib` crate's property tables and the GD Info Explorer schema.
//! Anything not wrapped here can still be built with `trigger(id, x, y)` + `set(...)`.

/// What kind of script value an argument accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum K {
    Num,
    Bool,
    Str,
}

#[derive(Debug)]
pub struct Arg {
    pub name: &'static str,
    pub key: u32,
    pub kind: K,
    pub optional: bool,
    /// Extra property written whenever this argument is given (e.g. an item id also needs its
    /// item *type*, which defaults to "counter").
    pub also: Option<(u32, &'static str)>,
    pub doc: &'static str,
}

#[derive(Debug)]
pub struct Def {
    pub name: &'static str,
    pub id: u32,
    pub doc: &'static str,
    pub args: &'static [Arg],
    /// Properties every fresh copy of this trigger carries (what the game writes by default).
    pub fixed: &'static [(u32, &'static str)],
}

const fn req(name: &'static str, key: u32, doc: &'static str) -> Arg {
    Arg { name, key, kind: K::Num, optional: false, also: None, doc }
}
const fn reqb(name: &'static str, key: u32, doc: &'static str) -> Arg {
    Arg { name, key, kind: K::Bool, optional: false, also: None, doc }
}
const fn opt(name: &'static str, key: u32, doc: &'static str) -> Arg {
    Arg { name, key, kind: K::Num, optional: true, also: None, doc }
}
const fn flag(name: &'static str, key: u32, doc: &'static str) -> Arg {
    Arg { name, key, kind: K::Bool, optional: true, also: None, doc }
}
const fn txt(name: &'static str, key: u32, doc: &'static str) -> Arg {
    Arg { name, key, kind: K::Str, optional: true, also: None, doc }
}
const fn reqi(name: &'static str, key: u32, type_key: u32, doc: &'static str) -> Arg {
    Arg { name, key, kind: K::Num, optional: false, also: Some((type_key, "1")), doc }
}
const fn item(name: &'static str, key: u32, type_key: u32, doc: &'static str) -> Arg {
    Arg { name, key, kind: K::Num, optional: true, also: Some((type_key, "1")), doc }
}

const EASE: Arg = opt("easing", 30, "easing curve, one of the EASE_* constants");
const RATE: Arg = opt("easing_rate", 85, "easing rate (the game's default is 2)");

/// Options every trigger accepts by name (never positionally).
pub static COMMON: &[Arg] = &[
    flag("spawn_triggered", 62, "trigger only fires when spawned"),
    flag("touch_triggered", 11, "trigger fires when the player touches it"),
    flag("multi_triggered", 87, "trigger can fire more than once"),
];

pub static TRIGGERS: &[Def] = &[
    // ---------------------------------------------------------- motion & transforms
    Def { name: "move", id: 901, doc: "Move a group by an offset.", fixed: &[], args: &[
        req("target", 51, "group to move"), req("dx", 28, "x offset in units (30 = 1 block)"),
        req("dy", 29, "y offset in units"), req("secs", 10, "duration in seconds"),
        EASE, RATE,
        flag("lock_player_x", 58, "follow the player's x movement"),
        flag("lock_player_y", 59, "follow the player's y movement"),
    ]},
    Def { name: "rotate", id: 1346, doc: "Rotate a group around a center group.", fixed: &[], args: &[
        req("target", 51, "group to rotate"), req("center", 71, "center group (0 = own center)"),
        req("degrees", 68, "degrees to rotate"), req("secs", 10, "duration in seconds"),
        EASE, RATE,
        opt("times360", 69, "extra full turns"),
        flag("lock_rotation", 70, "lock object rotation"),
    ]},
    Def { name: "scale", id: 2067, doc: "Scale a group around a center group.", fixed: &[], args: &[
        req("target", 51, "group to scale"), req("center", 71, "center group (0 = own center)"),
        req("scale_x", 150, "x scale factor"), req("scale_y", 151, "y scale factor"),
        req("secs", 10, "duration in seconds"), EASE, RATE,
        flag("only_move", 133, "only move objects, don't scale them"),
        flag("relative_scale", 577, "scale relative to the current scale"),
    ]},
    Def { name: "follow", id: 1347, doc: "Make a group follow another group.", fixed: &[], args: &[
        req("target", 51, "group that follows"), req("follow_group", 71, "group being followed"),
        req("x_mod", 72, "x movement multiplier"), req("y_mod", 73, "y movement multiplier"),
        req("secs", 10, "how long to follow"),
    ]},
    Def { name: "shake", id: 1520, doc: "Shake the screen.", fixed: &[], args: &[
        req("strength", 75, "shake strength"), req("interval", 84, "seconds between shakes"),
        req("secs", 10, "duration in seconds"),
    ]},
    Def { name: "animate", id: 1585, doc: "Play an animation on a group.", fixed: &[], args: &[
        req("target", 51, "group to animate"), req("animation", 76, "animation id"),
    ]},
    // ---------------------------------------------------------- visibility & control
    Def { name: "alpha", id: 1007, doc: "Fade a group to an opacity.", fixed: &[], args: &[
        req("target", 51, "group"), req("opacity", 35, "0 (invisible) to 1 (opaque)"),
        req("secs", 10, "duration in seconds"),
    ]},
    Def { name: "toggle", id: 1049, doc: "Turn a group on or off.", fixed: &[], args: &[
        req("target", 51, "group"), reqb("on", 56, "true = on, false = off"),
    ]},
    Def { name: "spawn", id: 1268, doc: "Spawn (trigger) a group of triggers.", fixed: &[], args: &[
        req("target", 51, "group to spawn"), req("delay", 63, "delay in seconds"),
        txt("remap", 442, "remap string, e.g. \"1.2.3.4\" remaps group 1 to 2 and 3 to 4"),
        flag("ordered", 441, "spawn ordered"), flag("reset_remap", 581, "reset remap"),
        opt("delay_var", 556, "random +/- delay variation"),
        flag("no_preview", 102, "disable editor preview"),
    ]},
    Def { name: "stop", id: 1616, doc: "Stop, pause or resume a group's triggers.", fixed: &[], args: &[
        req("target", 51, "group (or control id)"),
        opt("mode", 580, "STOP_STOP, STOP_PAUSE or STOP_RESUME"),
        flag("use_control_id", 535, "treat target as a control id"),
    ]},
    Def { name: "reset", id: 3618, doc: "Reset a group's trigger state.", fixed: &[], args: &[
        req("target", 51, "group"),
    ]},
    Def { name: "touch", id: 1595, doc: "Toggle a group when the player taps.", fixed: &[], args: &[
        req("target", 51, "group"), flag("hold", 81, "hold mode"),
        opt("toggle_mode", 82, "0 unset, 1 on, 2 off"), flag("dual", 89, "dual mode"),
    ]},
    // ---------------------------------------------------------- colour
    Def { name: "color_trigger", id: 899, doc: "Change a colour channel.", fixed: &[], args: &[
        req("channel", 23, "colour channel"), req("r", 7, "red 0-255"), req("g", 8, "green 0-255"),
        req("b", 9, "blue 0-255"), req("secs", 10, "fade time in seconds"),
        opt("opacity", 35, "0 to 1"), flag("blending", 17, "blending"),
        flag("player1", 15, "use player colour 1"), flag("player2", 16, "use player colour 2"),
    ]},
    Def { name: "pulse", id: 1006, doc: "Pulse a colour channel or group.", fixed: &[], args: &[
        req("target", 51, "colour channel (or group, see target_is_group)"),
        req("r", 7, "red 0-255"), req("g", 8, "green 0-255"), req("b", 9, "blue 0-255"),
        req("fade_in", 45, "seconds"), req("hold", 46, "seconds"), req("fade_out", 47, "seconds"),
        flag("target_is_group", 52, "target is a group, not a colour channel"),
        flag("exclusive", 86, "exclusive pulse"), flag("main_only", 65, "main colour only"),
        flag("detail_only", 66, "detail colour only"),
    ]},
    Def { name: "gradient", id: 2903, doc: "Define a gradient between four group vertices.",
        fixed: &[(202, "1"), (456, "1")], args: &[
        req("gradient_id", 209, "gradient id"),
        req("up", 203, "group of the up (bottom-left in vertex mode) vertex"),
        req("down", 204, "group of the down (bottom-right) vertex"),
        req("left", 205, "group of the left (top-left) vertex"),
        req("right", 206, "group of the right (top-right) vertex"),
        opt("layer", 202, "LAYER_* constant (default LAYER_MG)"),
        opt("blending", 174, "BLEND_* constant"),
        flag("vertex_mode", 207, "use vertex mode"), flag("disable", 208, "disable this gradient"),
        opt("preview_opacity", 456, "editor preview opacity 0-1"),
        flag("disable_all", 508, "disable all gradients"),
    ]},
    // ---------------------------------------------------------- camera
    Def { name: "camera_static", id: 1914, doc: "Lock the camera to a group.", fixed: &[], args: &[
        req("target", 71, "group to focus on"), req("secs", 10, "duration in seconds"),
        EASE, RATE, flag("follow", 212, "follow the group"),
        opt("follow_easing", 213, "follow easing"), flag("exit_instant", 465, "exit instantly"),
        flag("exit_static", 110, "exit static"),
    ]},
    Def { name: "camera_zoom", id: 1913, doc: "Zoom the camera.", fixed: &[], args: &[
        req("zoom", 371, "zoom factor (1 = normal)"), req("secs", 10, "duration in seconds"),
        EASE, RATE,
    ]},
    Def { name: "camera_rotate", id: 2015, doc: "Rotate the camera.", fixed: &[], args: &[
        req("degrees", 68, "degrees"), req("secs", 10, "duration in seconds"), EASE, RATE,
        flag("add", 70, "add to current rotation"), flag("snap360", 394, "snap to 360"),
    ]},
    Def { name: "camera_offset", id: 1916, doc: "Offset the camera.", fixed: &[], args: &[
        req("dx", 28, "x offset"), req("dy", 29, "y offset"), req("secs", 10, "duration in seconds"),
        EASE, RATE, opt("xy_only", 101, "XY_NEITHER, XY_X_ONLY or XY_Y_ONLY"),
    ]},
    Def { name: "camera_edge", id: 2062, doc: "Pin a camera edge to a group.", fixed: &[], args: &[
        req("edge", 164, "EDGE_LEFT, EDGE_RIGHT, EDGE_UP or EDGE_DOWN"),
        req("target", 51, "group"),
    ]},
    Def { name: "camera_mode", id: 2925, doc: "Switch camera free mode.", fixed: &[], args: &[
        reqb("free_mode", 111, "free camera mode"), flag("edit_settings", 112, "edit settings"),
        opt("camera_easing", 113, "camera easing"), opt("padding", 114, "padding 0-1"),
        flag("disable_gridsnap", 370, "disable grid snap"),
    ]},
    // ---------------------------------------------------------- audio
    Def { name: "song", id: 1934, doc: "Play a song.", fixed: &[], args: &[
        req("song", 392, "song id"), req("volume", 406, "volume 0-1"),
        opt("speed", 404, "speed index"), flag("loop", 413, "loop"),
        opt("start", 408, "start ms"), opt("end", 410, "end ms"),
        opt("fade_in", 409, "fade in ms"), opt("fade_out", 411, "fade out ms"),
        opt("channel", 432, "song channel"), flag("dont_reset", 595, "don't stop on death"),
    ]},
    Def { name: "sfx", id: 3602, doc: "Play a sound effect.", fixed: &[], args: &[
        req("sfx", 392, "sfx id"), req("volume", 406, "volume 0-1"),
        opt("speed", 404, "speed index"), opt("pitch", 405, "pitch"), flag("loop", 413, "loop"),
        opt("start", 408, "start ms"), opt("end", 410, "end ms"),
        opt("fade_in", 409, "fade in ms"), opt("fade_out", 411, "fade out ms"),
        opt("unique_id", 416, "unique id"), flag("is_unique", 415, "unique"),
        flag("override", 420, "override"), flag("reverb", 407, "reverb"),
        opt("min_interval", 434, "minimum interval"),
    ]},
    // ---------------------------------------------------------- items, counters, timers
    Def { name: "pickup", id: 1817, doc: "Change an item's count when the player passes.",
        fixed: &[(449, "1")], args: &[
        req("item", 80, "item id"), req("count", 77, "amount"),
        opt("mode", 88, "PICKUP_ADD, PICKUP_MULTIPLY or PICKUP_DIVIDE"),
        opt("modifier", 449, "modifier (default 1)"), flag("override", 139, "override value"),
    ]},
    Def { name: "count", id: 1611, doc: "Activate a group when an item reaches a count.", fixed: &[], args: &[
        req("item", 80, "item id"), req("target", 51, "group"), req("count", 77, "target count"),
        reqb("activate", 56, "true = activate group, false = deactivate"),
        flag("multi", 104, "multi activate"),
    ]},
    Def { name: "instant_count", id: 1811, doc: "Compare an item to a count right now.", fixed: &[], args: &[
        req("item", 80, "item id"), req("target", 51, "group"), req("count", 77, "count"),
        req("comparison", 88, "INSTANT_EQUALS, INSTANT_LARGER or INSTANT_SMALLER"),
        reqb("activate", 56, "true = activate group, false = deactivate"),
    ]},
    Def { name: "timer", id: 3614, doc: "Start a timer.", fixed: &[(470, "1")], args: &[
        req("timer", 80, "timer id"), req("start", 467, "start time in seconds"),
        opt("target", 51, "group to activate at stop time"), opt("stop", 473, "stop time"),
        flag("stop_enabled", 474, "stop at stop time"), opt("time_mod", 470, "time multiplier"),
        flag("ignore_timewarp", 469, "ignore timewarp"), flag("start_paused", 471, "start paused"),
        flag("dont_override", 468, "don't override"),
    ]},
    Def { name: "time_event", id: 3615, doc: "Activate a group when a timer reaches a time.", fixed: &[], args: &[
        req("timer", 80, "timer id"), req("target", 51, "group"), req("time", 473, "target time"),
        flag("multi", 475, "multi activate"),
    ]},
    Def { name: "time_control", id: 3617, doc: "Stop or start a timer.", fixed: &[], args: &[
        req("timer", 80, "timer id"), reqb("stop", 472, "true = stop, false = start"),
    ]},
    Def { name: "item_edit", id: 3619, doc: "Compute a value into an item (counter or timer).",
        fixed: &[(478, "1"), (481, "1"), (482, "3")], args: &[
        req("target", 51, "item to write to"),
        req("op", 480, "how to write: OP_SET, OP_ADD, OP_SUB, OP_MUL or OP_DIV"),
        req("modifier", 479, "number combined with the operands"),
        item("item1", 80, 476, "first operand item"), item("item2", 95, 477, "second operand item"),
        opt("item_op", 481, "operator between the two items (OP_ADD...)"),
        opt("mod_op", 482, "OP_MUL (default) or OP_DIV for the modifier"),
        opt("type1", 476, "ITEM_COUNTER, ITEM_TIMER, ITEM_POINTS, ITEM_TIME or ITEM_ATTEMPTS"),
        opt("type2", 477, "type of item2"), opt("target_type", 478, "type of the target item"),
        opt("round1", 485, "ROUND_* applied to the operand result"),
        opt("round2", 486, "ROUND_* applied to the final result"),
        opt("sign1", 578, "SIGN_* applied to the operand result"),
        opt("sign2", 579, "SIGN_* applied to the final result"),
    ]},
    Def { name: "item_compare", id: 3620, doc: "Spawn one of two groups depending on a comparison.",
        fixed: &[(479, "1"), (483, "1"), (480, "3"), (481, "3")], args: &[
        reqi("item1", 80, 476, "left item"), reqi("item2", 95, 477, "right item"),
        req("compare", 482, "CMP_EQ, CMP_GT, CMP_GTE, CMP_LT or CMP_LTE"),
        req("true_group", 51, "group spawned when true (0 = none)"),
        req("false_group", 71, "group spawned when false (0 = none)"),
        opt("mod1", 479, "multiplier for item1"), opt("mod2", 483, "multiplier for item2"),
        opt("tolerance", 484, "tolerance"),
        opt("type1", 476, "ITEM_* type of item1 (default counter)"),
        opt("type2", 477, "ITEM_* type of item2 (default counter)"),
        opt("round1", 485, "ROUND_* for the left side"), opt("round2", 486, "ROUND_* for the right side"),
        opt("sign1", 578, "SIGN_* for the left side"), opt("sign2", 579, "SIGN_* for the right side"),
    ]},
    // ---------------------------------------------------------- logic & collision
    Def { name: "random", id: 1912, doc: "Spawn one of two groups by chance.", fixed: &[], args: &[
        req("chance", 10, "percent chance of group_a (0-100)"),
        req("group_a", 51, "group on success"), req("group_b", 71, "group otherwise"),
    ]},
    Def { name: "adv_random", id: 2068, doc: "Spawn one of many groups by weight.", fixed: &[], args: &[
        Arg { name: "groups", key: 152, kind: K::Str, optional: false, also: None,
              doc: "\"group.weight.group.weight...\", e.g. \"1.5.2.3\"" },
    ]},
    Def { name: "on_death", id: 1812, doc: "Toggle a group when the player dies.", fixed: &[], args: &[
        req("target", 51, "group"), reqb("activate", 56, "true = activate"),
    ]},
    Def { name: "collision", id: 1815, doc: "Toggle a group when two collision blocks touch.", fixed: &[], args: &[
        req("block_a", 80, "first collision block id"), req("block_b", 95, "second collision block id"),
        req("target", 51, "group"), reqb("activate", 56, "true = activate"),
        flag("on_exit", 93, "trigger on exit"), flag("p1", 138, "player 1"),
        flag("p2", 200, "player 2"), flag("any_player", 201, "either player"),
    ]},
    Def { name: "instant_collision", id: 3609, doc: "Spawn a group depending on whether blocks touch.", fixed: &[], args: &[
        req("block_a", 80, "first collision block id"), req("block_b", 95, "second collision block id"),
        req("true_group", 51, "group when colliding"), req("false_group", 71, "group when not"),
        flag("p1", 138, "player 1"), flag("p2", 200, "player 2"), flag("any_player", 201, "either player"),
    ]},
    // ---------------------------------------------------------- world & player
    Def { name: "gravity", id: 2066, doc: "Change gravity.", fixed: &[], args: &[
        req("gravity", 148, "gravity multiplier"), flag("p1", 138, "player 1"),
        flag("p2", 200, "player 2"), flag("target_player", 201, "target player"),
    ]},
    Def { name: "timewarp", id: 1935, doc: "Change game speed.", fixed: &[], args: &[
        req("scale", 120, "time scale (1 = normal)"),
    ]},
    Def { name: "bg_speed", id: 3606, doc: "Change background scroll speed.", fixed: &[], args: &[
        req("mod_x", 143, "x multiplier"), req("mod_y", 144, "y multiplier"),
    ]},
    Def { name: "level_end", id: 3600, doc: "End the level.", fixed: &[], args: &[
        opt("spawn_group", 51, "group to spawn on end"), opt("target_pos", 71, "end position group"),
        flag("no_effects", 460, "no end effects"), flag("instant", 487, "instant"),
        flag("no_sfx", 461, "no end sound"),
    ]},
    Def { name: "show_player", id: 1613, doc: "Show the player.", fixed: &[], args: &[] },
    Def { name: "hide_player", id: 1612, doc: "Hide the player.", fixed: &[], args: &[] },
    Def { name: "bg_effect_on", id: 1818, doc: "Turn the background effect on.", fixed: &[], args: &[] },
    Def { name: "bg_effect_off", id: 1819, doc: "Turn the background effect off.", fixed: &[], args: &[] },
    Def { name: "reverse_gameplay", id: 1917, doc: "Reverse gameplay direction.", fixed: &[], args: &[] },
];

pub fn find(name: &str) -> Option<&'static Def> {
    TRIGGERS.iter().find(|d| d.name == name)
}

/// Named constants available in every script.
pub static CONSTS: &[(&str, f64)] = &[
    // easing (key 30)
    ("EASE_NONE", 0.0), ("EASE_IN_OUT", 1.0), ("EASE_IN", 2.0), ("EASE_OUT", 3.0),
    ("ELASTIC_IN_OUT", 4.0), ("ELASTIC_IN", 5.0), ("ELASTIC_OUT", 6.0),
    ("BOUNCE_IN_OUT", 7.0), ("BOUNCE_IN", 8.0), ("BOUNCE_OUT", 9.0),
    ("EXP_IN_OUT", 10.0), ("EXP_IN", 11.0), ("EXP_OUT", 12.0),
    ("SINE_IN_OUT", 13.0), ("SINE_IN", 14.0), ("SINE_OUT", 15.0),
    ("BACK_IN_OUT", 16.0), ("BACK_IN", 17.0), ("BACK_OUT", 18.0),
    // item edit / compare
    ("OP_SET", 0.0), ("OP_ADD", 1.0), ("OP_SUB", 2.0), ("OP_MUL", 3.0), ("OP_DIV", 4.0),
    ("CMP_EQ", 0.0), ("CMP_GT", 1.0), ("CMP_GTE", 2.0), ("CMP_LT", 3.0), ("CMP_LTE", 4.0),
    ("ITEM_COUNTER", 1.0), ("ITEM_TIMER", 2.0), ("ITEM_POINTS", 3.0), ("ITEM_TIME", 4.0),
    ("ITEM_ATTEMPTS", 5.0),
    ("ROUND_NONE", 0.0), ("ROUND_NEAREST", 1.0), ("ROUND_FLOOR", 2.0), ("ROUND_CEIL", 3.0),
    ("SIGN_NONE", 0.0), ("SIGN_ABS", 1.0), ("SIGN_NEG", 2.0),
    // pickup / count / stop
    ("PICKUP_ADD", 0.0), ("PICKUP_MULTIPLY", 1.0), ("PICKUP_DIVIDE", 2.0),
    ("INSTANT_EQUALS", 0.0), ("INSTANT_LARGER", 1.0), ("INSTANT_SMALLER", 2.0),
    ("STOP_STOP", 0.0), ("STOP_PAUSE", 1.0), ("STOP_RESUME", 2.0),
    // camera
    ("XY_NEITHER", 0.0), ("XY_X_ONLY", 1.0), ("XY_Y_ONLY", 2.0),
    ("EDGE_LEFT", 1.0), ("EDGE_RIGHT", 2.0), ("EDGE_UP", 3.0), ("EDGE_DOWN", 4.0),
    // gradient
    ("BLEND_NORMAL", 0.0), ("BLEND_ADDITIVE", 1.0), ("BLEND_MULTIPLY", 2.0), ("BLEND_INVERT", 3.0),
    ("LAYER_BG", 0.0), ("LAYER_MG", 1.0), ("LAYER_B5", 2.0), ("LAYER_B4", 3.0), ("LAYER_B3", 4.0),
    ("LAYER_B2", 5.0), ("LAYER_B1", 6.0), ("LAYER_P", 7.0), ("LAYER_T1", 8.0), ("LAYER_T2", 9.0),
    ("LAYER_T3", 10.0), ("LAYER_T4", 11.0), ("LAYER_G", 12.0), ("LAYER_UI", 13.0),
    // colour channels with special meaning
    ("CH_BG", 1000.0), ("CH_GROUND", 1001.0), ("CH_LINE", 1002.0), ("CH_3DL", 1003.0),
    ("CH_OBJECT", 1004.0), ("CH_P1", 1005.0), ("CH_P2", 1006.0), ("CH_GROUND2", 1009.0),
];

/// Generates `docs/TRIGGERS.md`. A test keeps the committed file in sync with the table.
pub fn markdown() -> String {
    let mut s = String::new();
    s.push_str("# Trigger reference\n\n");
    s.push_str("<!-- Generated by `gdl docs triggers`. Do not edit by hand. -->\n\n");
    s.push_str(
        "Every trigger is called as `name(x, y, ...)` and returns an object handle.\n\
         Required arguments come first, in the order shown; optional ones follow and are\n\
         easiest to give **by name**:\n\n\
         ```\n\
         pickup(300, 15, 1, 5)                         // item 1 += 5\n\
         pickup(300, 15, 1, 2, mode = PICKUP_MULTIPLY) // item 1 *= 2\n\
         move(0, 45, 3, 0, 60, 2, easing = EASE_IN_OUT, easing_rate = 1.5)\n\
         ```\n\n",
    );
    s.push_str("Every trigger also accepts these options by name: ");
    let common: Vec<String> = COMMON.iter().map(|a| format!("`{}`", a.name)).collect();
    s.push_str(&common.join(", "));
    s.push_str(".\n\nNeed one that isn't here? `trigger(id, x, y)` makes any trigger and `set(h, key, value)` fills in properties (see `P_*` and `ID_*` constants).\n\n");
    s.push_str(&format!("{} triggers are wrapped.\n\n", TRIGGERS.len()));
    s.push_str("| trigger | object id | signature |\n|---|---|---|\n");
    for d in TRIGGERS {
        s.push_str(&format!("| [`{}`](#{}) | {} | `{}` |\n", d.name, d.name, d.id, signature(d)));
    }
    for d in TRIGGERS {
        s.push_str(&format!("\n## {}\n\n{} Object id `{}`.\n\n", d.name, d.doc, d.id));
        s.push_str(&format!("```\n{}\n```\n\n", signature(d)));
        if !d.args.is_empty() {
            s.push_str("| argument | key | | description |\n|---|---|---|---|\n");
            for a in d.args {
                let kind = match a.kind {
                    K::Num => "number",
                    K::Bool => "true/false",
                    K::Str => "text",
                };
                s.push_str(&format!(
                    "| `{}` | {} | {}{} | {} |\n",
                    a.name,
                    a.key,
                    kind,
                    if a.optional { ", optional" } else { "" },
                    a.doc
                ));
            }
        }
    }
    s.push_str("\n## Constants\n\n");
    let mut line = String::new();
    for (n, v) in CONSTS {
        let item = format!("`{}`={}", n, *v as i64);
        if line.len() + item.len() > 90 {
            s.push_str(&line);
            s.push_str("  \n");
            line.clear();
        }
        line.push_str(&item);
        line.push(' ');
    }
    s.push_str(&line);
    s.push('\n');
    s
}

fn signature(d: &Def) -> String {
    let mut v = vec!["x".to_string(), "y".to_string()];
    for a in d.args {
        v.push(if a.optional { format!("[{}]", a.name) } else { a.name.to_string() });
    }
    format!("{}({})", d.name, v.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_required_args_come_first() {
        for (i, d) in TRIGGERS.iter().enumerate() {
            assert!(TRIGGERS[..i].iter().all(|o| o.name != d.name), "duplicate {}", d.name);
            let mut seen_opt = false;
            for a in d.args {
                if a.optional {
                    seen_opt = true;
                } else {
                    assert!(!seen_opt, "{}: required '{}' after an optional arg", d.name, a.name);
                }
            }
            let mut names: Vec<&str> = d.args.iter().map(|a| a.name).collect();
            names.sort();
            names.dedup();
            assert_eq!(names.len(), d.args.len(), "{}: duplicate arg name", d.name);
        }
    }

    #[test]
    fn committed_docs_match_the_table() {
        let committed = include_str!("../../docs/TRIGGERS.md");
        assert_eq!(
            committed,
            markdown(),
            "docs/TRIGGERS.md is out of date: run `cargo run -p gdl -- docs triggers > docs/TRIGGERS.md`"
        );
    }
}
