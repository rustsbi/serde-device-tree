use crate::buildin::Matrix;

use serde_derive::Serialize;

use core::ops::RangeInclusive;

#[repr(transparent)]
#[derive(Serialize)]
#[serde(transparent)]
pub struct EventToMhpmevent<'de>(Matrix<'de, 3>);

#[repr(transparent)]
#[derive(Serialize)]
#[serde(transparent)]
pub struct EventToMhpmcounters<'de>(Matrix<'de, 3>);

#[repr(transparent)]
#[derive(Serialize)]
#[serde(transparent)]
pub struct RawEventToMhpcounters<'de>(Matrix<'de, 5>);

impl EventToMhpmevent<'_> {
    pub fn get_len(&self) -> usize {
        self.0.len()
    }

    pub fn get_event_id(&self, i: usize) -> u32 {
        u32::from_be(self.0.get(i)[0])
    }

    pub fn get_selector_value(&self, i: usize) -> u64 {
        let current = self.0.get(i);
        ((u32::from_be(current[1]) as u64) << 32) | (u32::from_be(current[0]) as u64)
    }
}

impl EventToMhpmcounters<'_> {
    pub fn get_len(&self) -> usize {
        self.0.len()
    }

    pub fn get_event_idx_range(&self, i: usize) -> RangeInclusive<u32> {
        let current = self.0.get(i);
        u32::from_be(current[0])..=u32::from_be(current[1])
    }

    pub fn get_counter_bitmap(&self, i: usize) -> u32 {
        let current = self.0.get(i);
        u32::from_be(current[2])
    }
}

impl RawEventToMhpcounters<'_> {
    pub fn get_len(&self) -> usize {
        self.0.len()
    }

    pub fn get_event_idx_base(&self, i: usize) -> u64 {
        let current = self.0.get(i);
        ((u32::from_be(current[0]) as u64) << 32) | (u32::from_be(current[1]) as u64)
    }

    pub fn get_event_idx_mask(&self, i: usize) -> u64 {
        let current = self.0.get(i);
        ((u32::from_be(current[2]) as u64) << 32) | (u32::from_be(current[3]) as u64)
    }

    pub fn get_counter_bitmap(&self, i: usize) -> u32 {
        let current = self.0.get(i);
        u32::from_be(current[4])
    }
}
