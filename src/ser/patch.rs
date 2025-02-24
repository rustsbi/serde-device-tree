use super::serializer::Serializer;
use core::cell::Cell;

pub struct Patch<'se> {
    name: &'se str,
    pub data: &'se dyn erased_serde::Serialize,
    matched_depth: Cell<usize>,
    parsed: Cell<bool>,
}

impl<'se> Patch<'se> {
    pub fn new(name: &'se str, data: &'se dyn erased_serde::Serialize) -> Patch<'se> {
        Patch {
            name,
            data,
            matched_depth: Cell::new(0),
            parsed: Cell::new(false),
        }
    }

    pub fn init(&self) {
        self.matched_depth.set(0);
        self.parsed.set(false);
    }

    pub fn get_depth(&self) -> usize {
        self.name.split('/').count() - 1
    }

    pub fn get_depth_path(&self, x: usize) -> &'se str {
        if x == 0 {
            return "";
        }
        match self.name.split('/').nth(x) {
            Some(result) => result,
            None => "",
        }
    }

    // I hope to impl serde::ser::Serializer, but erase_serialize's return value is different from
    // normal serialize, so we do this.
    pub fn serialize(&self, serializer: &mut Serializer<'se>) {
        self.parsed.set(true);
        self.data
            .erased_serialize(&mut <dyn erased_serde::Serializer>::erase(serializer))
            .unwrap();
    }
}

pub struct PatchList<'se> {
    list: &'se [Patch<'se>],
}

impl<'se> PatchList<'se> {
    pub fn new(list: &'se [Patch<'se>]) -> PatchList<'se> {
        PatchList { list }
    }

    pub fn step_forward(&self, name: &'se str, depth: usize) -> Option<&'se Patch<'se>> {
        let mut matched_patch = None;
        self.list.iter().for_each(|patch| {
            if patch.matched_depth.get() == depth - 1 && patch.get_depth_path(depth) == name {
                patch.matched_depth.set(patch.matched_depth.get() + 1);
                if patch.get_depth() == depth {
                    if let Some(_) = matched_patch {
                        panic!("More than one replace data on a same path");
                    }
                    matched_patch = Some(patch);
                }
            }
        });
        matched_patch
    }

    pub fn step_back(&self, depth: usize) {
        self.list.iter().for_each(|patch| {
            if patch.matched_depth.get() == depth {
                patch.matched_depth.set(patch.matched_depth.get() - 1);
            }
        });
    }

    pub fn add_list(&self, depth: usize) -> impl Iterator<Item = &'se Patch<'se>> + use<'se> {
        self.list.iter().filter(move |x| {
            x.matched_depth.get() == depth && x.get_depth() == depth + 1 && x.parsed.get() == false
        })
    }
}
