use core::cell::Cell;

pub struct Patch<'de> {
    name: &'de str,
    pub data: &'de dyn erased_serde::Serialize,
    matched_depth: Cell<usize>,
    parsed: Cell<bool>,
}

impl<'de> Patch<'de> {
    pub fn new(name: &'de str, data: &'de dyn erased_serde::Serialize) -> Patch<'de> {
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

    pub fn get_depth_path(&self, x: usize) -> &'de str {
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
    pub fn serialize(&self, serializer: &mut crate::ser::Serializer<'de>) {
        self.parsed.set(true);
        self.data
            .erased_serialize(&mut <dyn erased_serde::Serializer>::erase(serializer))
            .unwrap();
    }
}

pub struct PatchList<'de> {
    list: &'de [Patch<'de>],
}

impl<'de> PatchList<'de> {
    pub fn new(list: &'de [Patch<'de>]) -> PatchList<'de> {
        PatchList { list }
    }

    pub fn step_forward(&self, name: &'de str, depth: usize) -> Option<&'de Patch<'de>> {
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
}
