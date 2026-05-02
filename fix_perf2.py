import re

path = 'crates/autoinstinct/tests/anti_fake_perf.rs'
with open(path, 'r') as f:
    c = f.read()

# Insert `let snap_arc = std::sync::Arc::new(snap.clone());` after `let (snap, posture, ctx) = closed_surface(&s);`
c = c.replace('let (snap, posture, ctx) = closed_surface(&s);', 'let (snap, posture, ctx) = closed_surface(&s);\n        let snap_arc = std::sync::Arc::new(snap.clone());')
c = c.replace('let (snap, posture, ctx) = closed_surface(&s0);', 'let (snap, posture, ctx) = closed_surface(&s0);\n        let snap_arc = std::sync::Arc::new(snap.clone());')

# Replace `std::sync::Arc::new(snap.clone())` with `snap_arc.clone()`
c = c.replace('std::sync::Arc::new(snap.clone())', 'snap_arc.clone()')

# There is one with `s0`: `std::sync::Arc::new(s0.clone())`? No, closed_surface returns `snap`.

with open(path, 'w') as f:
    f.write(c)
