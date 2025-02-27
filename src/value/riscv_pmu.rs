/// This module implement prop value described in
/// https://www.kernel.org/doc/Documentation/devicetree/bindings/perf/riscv%2Cpmu.yaml
use crate::buildin::*;

use serde::{Deserialize, Serialize};

use core::ops::RangeInclusive;

#[repr(transparent)]
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct EventToMhpmevent<'a>(Matrix<'a, 3>);

#[repr(transparent)]
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct EventToMhpmcounters<'a>(Matrix<'a, 3>);

#[repr(transparent)]
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct RawEventToMhpcounters<'a>(Matrix<'a, 5>);

impl EventToMhpmevent<'_> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline(always)]
    pub fn get_event_id(&self, i: usize) -> u32 {
        u32::from_be(self.0.get(i)[0])
    }

    #[inline(always)]
    pub fn get_selector_value(&self, i: usize) -> u64 {
        let current = self.0.get(i);
        ((u32::from_be(current[1]) as u64) << 32) | (u32::from_be(current[2]) as u64)
    }
}

impl EventToMhpmcounters<'_> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline(always)]
    pub fn get_event_idx_range(&self, i: usize) -> RangeInclusive<u32> {
        let current = self.0.get(i);
        u32::from_be(current[0])..=u32::from_be(current[1])
    }

    #[inline(always)]
    pub fn get_counter_bitmap(&self, i: usize) -> u32 {
        let current = self.0.get(i);
        u32::from_be(current[2])
    }
}

impl RawEventToMhpcounters<'_> {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline(always)]
    pub fn get_event_idx_base(&self, i: usize) -> u64 {
        let current = self.0.get(i);
        ((u32::from_be(current[0]) as u64) << 32) | (u32::from_be(current[1]) as u64)
    }

    #[inline(always)]
    pub fn get_event_idx_mask(&self, i: usize) -> u64 {
        let current = self.0.get(i);
        ((u32::from_be(current[2]) as u64) << 32) | (u32::from_be(current[3]) as u64)
    }

    #[inline(always)]
    pub fn get_counter_bitmap(&self, i: usize) -> u32 {
        let current = self.0.get(i);
        u32::from_be(current[4])
    }
}

#[cfg(test)]
mod tests {
    use super::EventToMhpmcounters;
    use crate::{Dtb, DtbPtr, buildin::Node, from_raw_mut};

    const RAW_DEVICE_TREE: &[u8] = include_bytes!("../../examples/qemu-virt.dtb");
    const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();
    #[test]
    fn test_chosen_stdout() {
        #[repr(align(8))]
        struct AlignedBuffer {
            pub data: [u8; RAW_DEVICE_TREE.len()],
        }
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Node = from_raw_mut(&dtb).unwrap();
        let result = node
            .find("/pmu")
            .unwrap()
            .get_prop("riscv,event-to-mhpmcounters")
            .unwrap()
            .deserialize::<EventToMhpmcounters>();
        assert_eq!(result.len(), 5);
        assert_eq!(result.get_event_idx_range(0), 1..=1);
        assert_eq!(result.get_counter_bitmap(0), 0x7fff9);
        assert_eq!(result.get_event_idx_range(1), 2..=2);
        assert_eq!(result.get_counter_bitmap(1), 0x7fffc);
        assert_eq!(result.get_event_idx_range(2), 0x10019..=0x10019);
        assert_eq!(result.get_counter_bitmap(2), 0x7fff8);
    }
}
