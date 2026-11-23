//! Comparing projects across replicas.
//!
//! Engine ids are data (the crate docs), so two replicas that converged hold the very same ids;
//! what differs is each one's id counter ([`kition_motion_project::Project::next_id`]), which
//! bounds the ids that replica may mint next, and each machine's own render state
//! ([`crate::local`]): neither is replicated.

use kition_motion_project::Project;

use crate::local;

/// `project` with its id counter at 0 and no machine's render state, for comparisons across
/// replicas and round trips.
pub fn normalize(project: &Project) -> Project {
    let mut out = project.clone();
    out.next_id = 0;
    local::strip(&mut out);
    out
}

/// Whether `a` and `b` hold the same content, their id counters and render state aside.
/// Compositions that are one `Arc` compare by pointer first.
pub fn same_content(a: &Project, b: &Project) -> bool {
    let Project { schema, settings, items, next_id: _, render_queue, render_templates, render_prefs: _ } = a;
    let queue = |p: &Project| {
        let mut p = Project { render_queue: p.render_queue.clone(), ..Project::default() };
        local::strip(&mut p);
        p.render_queue
    };
    *schema == b.schema
        && *settings == b.settings
        && render_queue.len() == b.render_queue.len()
        && queue(a) == queue(b)
        && *render_templates == b.render_templates
        && items.len() == b.items.len()
        && items.iter().zip(&b.items).all(|((ka, x), (kb, y))| {
            ka == kb
                && match (&x.kind, &y.kind) {
                    (kition_motion_project::ItemKind::Comp(p), kition_motion_project::ItemKind::Comp(q)) => {
                        (std::sync::Arc::ptr_eq(p, q) || p == q)
                            && x.id == y.id
                            && x.name == y.name
                            && x.label == y.label
                            && x.comment == y.comment
                            && x.parent == y.parent
                            && x.proxy == y.proxy
                    }
                    _ => x == y,
                }
        })
}
