# Aporic government world

A small, local WebGL prototype of the advisory Aporic government. The player is
the president. Walk between three ministries, the prime minister, and the
independent inspector; select a person or building to inspect its current role.

Run from the repository root:

```sh
python3 -m http.server 8765 --directory world
```

Then open `http://127.0.0.1:8765` in a WebGL-capable browser. Use W/A/S/D or
the arrow keys to walk, drag the scene to rotate, and select a location or
person to reveal a card with a travel button. The map button switches between
the president view and an overview. Touch controls appear at narrow widths.

`government-data.js` is a fixed snapshot of the version 2 charter and the nine
incumbents verified on 2026-09-26 through `aporic_government_get` and
`aporic_government_roster`. Its 09:59 UTC status counts came from
`aporic_product_cell_list` and `aporic_task_list`. The task registry contains
advisory coordination records, including entries whose status may lag work in
Git. It is intentionally labelled as a dated snapshot. To reflect later
changes, refresh that file against the current Aporic record and verify names,
titles, and counts again. The prototype does not grant authority, execute
roles, or fetch live government data.

The scene uses a small local WebGL renderer and has no runtime package or
network dependency. The snapshot, scene, and interface can all be edited
without changing the Rust service.
