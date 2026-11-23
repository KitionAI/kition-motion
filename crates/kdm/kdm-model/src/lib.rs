//! Bridges the Motion engine's [`Project`] ([`kition_motion_project`]) to a `kit.motion` KDM
//! replica.
//!
//! [`MotionModel`] implements [`KitModel`]: it is the engine's materialised view of a replica, the
//! same [`Project`] the rest of the engine already knows. Items, compositions, layers, every group
//! and property of a layer's property tree, every keyframe and every Render Queue item become KDM
//! objects of their own, so concurrent edits to different ones merge finely; [`write_document`]
//! and [`write_changes`] are the engine command bridge's entry points: writing a whole project in
//! one creating transaction, or only what a command changed.
//!
//! **Ids are data.** Motion's own ids (`ItemId`, `LayerId`, property uids, Graphics Templates
//! control ids: everything [`Project::alloc`] hands out) are written as fields of their objects
//! and read back exactly, so every replica — and every reload — names an object by the same id.
//! References a model field holds by engine id deep inside a value (a `Value::Layer` of a Layer
//! Control, a Graphics Templates link, a media-replacement value, a tracker's target) therefore
//! stay valid across replicas without translation. What keeps two collaborators from minting the
//! same id before they exchange anything is that each actor mints from its own slot of the id
//! space ([`slot_base`]); the references the merge must check or repair (a layer's source item,
//! its parent, its track matte, a Render Queue item's composition, an item's folder) are KDM
//! `ref` fields naming the target's object, translated back to engine ids on read.
//!
//! **Everything is reached from the root** (KHP A16: an agent reads objects by following fields
//! from the root): `Project.items` lists every item as a query view. The project panel
//! sorts folders then names and stores no manual order; moving these listings is refused.
//! The folder tree is each item's `parent` reference;
//! an item's `comp` names its composition, a composition's `layers` its layers, a layer's
//! `props` its property tree, a property's `keys` its keyframes, a composition's and a layer's
//! `markers` their markers, a composition's `guides` its guides and `controls` its Graphics
//! Templates controls (a group's `children` its own), and `Project.renderQueue` the Render
//! Queue. The writer lists and unlists items in the same transaction;
//! the repair keeps one listing per live item. A replica of an earlier schema is refused (there is
//! no migration; its room is opened again from the `.kmotion` file).
//!
//! **Where each field lives** (`model/schema.json`): a struct's own fields are a KDM `map` (one
//! register per field: two people changing a composition's duration and its background colour
//! both keep their change), except the fields that are objects, lists or references of their
//! own, and an item's and a layer's `name`, a `string` field of its own an agent renames with one
//! `set`. A layer's switches are a map of their own (one register per switch). Markers, guides
//! and template controls are objects of their own, so two people adding one each keep both; a
//! marker's comment is a `text` field, merged by character. Footage is referenced by path, never
//! embedded (no blobs). What one machine's renders leave on the Render Queue (statuses) and its
//! Render Queue preferences are not replicated (`local`). Keyframes are a list per
//! property; they have no ids in the engine, so a write aligns the old and new key lists (by
//! content, then time, then value) and only a real insert or delete creates or deletes an object.
//!
//! **Tick range.** A layer's start, in and out points, every keyframe's time and every marker's
//! time and duration are decimal strings, exact at any `Tick`. Other ticks inside a register (a
//! composition's duration and work area) stay the engine's own JSON numbers: a KDM value cannot hold an integer past
//! ±2^53, about 9.85 hours at `kition_motion_time::TICKS_PER_SECOND`; such a project fails to
//! replicate with an error (never a panic or silent corruption), the same residual limit Kition
//! Film documents.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod compare;
mod edits;
mod fields;
mod ids;
mod list;
mod local;
mod read;
mod session;
mod settle;
mod tick;
mod validate;
mod write;

use std::collections::BTreeSet;
use std::sync::Arc;

use kition_kit::kdm::{ActorId, DocId, KdmError, ObjectId, Origin, Replica, Schema, Tx};
use kition_kit::session::{BlobSink, BlobSource, Damage, KitError, KitModel};
use kition_motion_project::Project;

pub use compare::{normalize, same_content};
pub use edits::check_model_edits;
pub use ids::{CompParts, ControlKey, Ids, LayerIds, LayerKey, NodeKey, layer_keys};
pub use read::read_document;
pub use session::{MotionSession, slot_base};
pub use settle::settle_batch;
pub use validate::repair_id;
pub use write::{write_changes, write_document};

/// Re-exported so downstream crates (the engine command bridge) share this crate's exact pin,
/// rather than taking their own dependency on `kition-sdk` that could drift from it.
pub use kition_kit;

type Result<T> = kition_kit::session::Result<T>;
pub(crate) type KdmResult<T> = std::result::Result<T, KdmError>;

const SCHEMA: &str = include_str!("../model/schema.json");

/// How deep a property tree may nest (groups within groups) before a write or a read refuses it.
pub(crate) const MAX_DEPTH: usize = 256;

/// The `Project` object's id: one per replica, minted by [`write_document`] with
/// [`Tx::create_with_id`], so every reader finds it without an index.
pub(crate) fn project_id() -> ObjectId {
    ObjectId::from_string("project")
}

/// Wraps anything `Display`s as a [`KitError`].
fn kit_err(error: impl std::fmt::Display) -> KitError {
    KitError::new(error.to_string())
}

/// Wraps anything `Display`s (a `serde_json::Error`, a `ParseIntError`, …) as a [`KdmError`].
pub(crate) fn kdm_err(error: impl std::fmt::Display) -> KdmError {
    KdmError::Engine(error.to_string())
}

/// The engine's view of a `kit.motion` replica: the same [`Project`] the rest of the engine
/// knows, plus the object ids [`write_changes`] needs to find each object again.
pub struct MotionModel {
    pub project: Project,
    ids: Ids,
}

impl MotionModel {
    /// A view built by the caller itself: `project` as written by [`write_document`] or
    /// [`write_changes`], with the ids they filled.
    pub fn from_parts(project: Project, ids: Ids) -> Self {
        Self { project, ids }
    }

    /// Replaces the view after the caller wrote `project` to the replica itself, with the ids
    /// that write left.
    pub fn adopt(&mut self, project: Project, ids: Ids) {
        self.project = project;
        self.ids = ids;
    }

    /// The object ids, current as of the last `load`/`apply`/`adopt`. A bridge clones them into
    /// its own working copy while it builds a [`write_changes`] transaction.
    pub fn ids(&self) -> &Ids {
        &self.ids
    }
}

impl std::fmt::Debug for MotionModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MotionModel").field("items", &self.project.items.len()).finish_non_exhaustive()
    }
}

/// A new replica of `actor` holding `project`, written in its creating transaction (not an
/// undo step); also answers the ids that write minted.
pub(crate) fn create_replica(project: &Project, actor: ActorId, origin: &Origin) -> KdmResult<(Replica, Ids)> {
    let mut ids = Ids::default();
    let (replica, ()) = Replica::create_with(MotionModel::schema(), DocId::new(), actor, origin, |tx| write_document(tx, project, &mut ids).map(|_| ()))?;
    Ok((replica, ids))
}

impl KitModel for MotionModel {
    fn schema() -> Arc<Schema> {
        #[allow(clippy::expect_used)]
        Arc::new(Schema::parse(SCHEMA).expect("model/schema.json is a fixed, tested file"))
    }

    fn load(replica: &Replica, _blobs: &dyn BlobSource) -> Result<Self> {
        let (project, ids) = read::read_document(replica).map_err(kit_err)?;
        Ok(Self { project, ids })
    }

    /// Reloads the whole view rather than patching the changed objects in place: a correct,
    /// simple fallback the `KitModel` contract allows. Ids are data, so a reload keeps every
    /// surviving object's engine id (and with it a session's selection) as it was.
    ///
    /// This machine's render state ([`local`]: not in the replica) stays as it was.
    ///
    /// The id counter never goes down: an id this replica handed out may be gone from the view
    /// (deleted by a peer, or undone) yet still live on a peer or in an undo step, so minting it
    /// again would give two objects one id.
    fn apply(&mut self, replica: &Replica, _changed: &BTreeSet<ObjectId>, _blobs: &dyn BlobSource) -> Result<Damage> {
        let (mut project, ids) = read::read_document(replica).map_err(kit_err)?;
        local::keep(&self.project, &mut project);
        let slot = slot_base(replica.actor());
        if (slot..slot.saturating_add(session::SLOT_SIZE)).contains(&self.project.next_id) {
            project.next_id = project.next_id.max(self.project.next_id);
        }
        self.project = project;
        self.ids = ids;
        Ok(Damage::Full)
    }

    fn validate(tx: &mut Tx<'_>, changed: &BTreeSet<ObjectId>) -> Result<bool> {
        validate::validate(tx, changed).map_err(kit_err)
    }

    fn import(_name: &str, bytes: &[u8], _blobs: &mut dyn BlobSink, actor: ActorId) -> Result<Replica> {
        let text = std::str::from_utf8(bytes).map_err(|_| KitError::new("not a text project file"))?;
        let project = Project::from_json(text).map_err(kit_err)?;
        let (replica, _) = create_replica(&project, actor, &Origin::system("import")).map_err(kit_err)?;
        Ok(replica)
    }

    fn export(&self, _replica: &Replica, _name: &str, _blobs: &dyn BlobSource) -> Result<Vec<u8>> {
        Ok(self.project.to_file_json().map_err(kit_err)?.into_bytes())
    }
}
