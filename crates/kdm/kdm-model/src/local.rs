//! What one machine keeps of a project that its replica does not hold: the Render Queue
//! preferences (notify when done, storage overflow folders: paths on this machine) and what a
//! render on this machine left on each Render Queue item (its status, when it started, how long
//! it took, where it wrote). The replica holds the queue's definition (what to render, and how);
//! a collaborator's render never changes another's statuses, and a machine that has not rendered
//! an item shows it queued (its Render box checked) or unqueued.
//!
//! [`keep`] carries this state across a reload of the view from the replica; [`strip`] removes
//! it, for comparing projects by what they replicate.

use std::collections::HashMap;

use kition_motion_project::Project;
use kition_motion_project::render_queue::RenderQueueItem;

use crate::fields::unrendered;

/// Copies `from`'s machine state into `into` (a project just read from the replica): the
/// Render Queue preferences, and each render item's render state by its id.
pub(crate) fn keep(from: &Project, into: &mut Project) {
    into.render_prefs = from.render_prefs.clone();
    let rendered: HashMap<u64, &RenderQueueItem> = from.render_queue.iter().map(|r| (r.id, r)).collect();
    for item in &mut into.render_queue {
        let Some(old) = rendered.get(&item.id) else { continue };
        // A status this machine set for an item whose Render box someone else changed since
        // follows the box, as checking it does here.
        item.status = if old.render == item.render { old.status.clone() } else { unrendered(item.render) };
        item.started = old.started;
        item.render_time = old.render_time;
        item.last_output = old.last_output.clone();
    }
}

/// `project` without any machine's state: default Render Queue preferences, and every render
/// item as a machine that has not rendered it shows it.
pub(crate) fn strip(project: &mut Project) {
    project.forget_machine_state();
}
