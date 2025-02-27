use super::serializer::Serializer;
use core::cell::Cell;

/// Since this crate is mostly work with `noalloc`, we use `Patch` and `PatchList` for change or
/// add on a dtb.
pub struct Patch<'se> {
    pub data: &'se dyn dyn_serde::Serialize,
    name: &'se str,

    /// This patch match how many item between its path and serializer.
    matched_depth: Cell<usize>,
    /// Show this patch have been parsed.
    parsed: Cell<bool>,
}

impl<'se> Patch<'se> {
    #[inline(always)]
    pub fn new(name: &'se str, data: &'se dyn dyn_serde::Serialize) -> Patch<'se> {
        Patch {
            name,
            data,
            matched_depth: Cell::new(0),
            parsed: Cell::new(false),
        }
    }

    #[inline(always)]
    /// Reset the status of patch.
    pub fn init(&self) {
        self.matched_depth.set(0);
        self.parsed.set(false);
    }

    #[inline(always)]
    pub fn get_depth(&self) -> usize {
        self.name.split('/').count() - 1
    }

    #[inline(always)]
    pub fn get_depth_path(&self, x: usize) -> &'se str {
        if x == 0 {
            return "";
        }
        self.name.split('/').nth(x).unwrap_or_default()
    }

    // I hope to impl serde::ser::Serializer, but erase_serialize's return value is different from
    // normal serialize, so we do this.
    /// Serialize this patch with serializer.
    #[inline(always)]
    pub fn serialize(&self, serializer: &mut Serializer<'se>) {
        self.parsed.set(true);
        self.data
            .serialize_dyn(&mut <dyn dyn_serde::Serializer>::new(serializer))
            .unwrap();
    }
}

/// Here is a list of `Patch`, and have some methods for update `Patch` status.
pub struct PatchList<'se> {
    list: &'se [Patch<'se>],
}

impl<'se> PatchList<'se> {
    #[inline(always)]
    pub fn new(list: &'se [Patch<'se>]) -> PatchList<'se> {
        PatchList { list }
    }

    #[inline(always)]
    pub fn step_forward(&self, name: &'se str, depth: usize) -> Option<&'se Patch<'se>> {
        let mut matched_patch = None;
        self.list.iter().for_each(|patch| {
            if patch.matched_depth.get() == depth - 1 && patch.get_depth_path(depth) == name {
                patch.matched_depth.set(patch.matched_depth.get() + 1);
                if patch.get_depth() == depth {
                    if matched_patch.is_some() {
                        panic!("More than one replace data on a same path");
                    }
                    matched_patch = Some(patch);
                }
            }
        });
        matched_patch
    }

    #[inline(always)]
    pub fn step_back(&self, depth: usize) {
        self.list.iter().for_each(|patch| {
            if patch.matched_depth.get() == depth {
                patch.matched_depth.set(patch.matched_depth.get() - 1);
            }
        });
    }

    #[inline(always)]
    /// Return a list which is on this level, but haven't been parsed, which usually means this
    /// patch is for adding.
    pub fn add_list(&self, depth: usize) -> impl Iterator<Item = &'se Patch<'se>> + use<'se> {
        self.list.iter().filter(move |x| {
            x.matched_depth.get() == depth && x.get_depth() == depth + 1 && !x.parsed.get()
        })
    }
}
