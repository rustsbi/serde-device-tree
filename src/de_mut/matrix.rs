use crate::de_mut::ValueCursor;
use serde::{Deserialize, Serialize};

pub struct Matrix<'de, const T: usize> {
    data: &'de [u32],
}

pub struct MatrixItem<'de, const T: usize> {
    offset: usize,
    data: &'de [u32],
}

impl<'de, const T: usize> Matrix<'de, T> {
    // Block size in bytes.
    pub fn get_block_size() -> usize {
        T * 4
    }

    pub fn iter(&self) -> MatrixItem<'de, T> {
        MatrixItem {
            offset: 0,
            data: self.data,
        }
    }
}

impl<'de, const T: usize> Iterator for MatrixItem<'de, T> {
    type Item = &'de [u32];

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() <= self.offset {
            return None;
        }
        let result = &self.data[self.offset..self.offset + T];
        self.offset += T;
        Some(result)
    }
}

impl<'de, const T: usize> Deserialize<'de> for Matrix<'de, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value_deserialzer = super::ValueDeserializer::deserialize(deserializer)?;
        let data = match value_deserialzer.cursor {
            ValueCursor::Prop(_, cursor) => cursor.data_on(value_deserialzer.dtb),
            _ => unreachable!(),
        };
        if data.len() % Self::get_block_size() != 0 {
            panic!("unaligned matrix");
        }
        let (prefix, data, suffix) = unsafe { data.align_to::<u32>() };
        if prefix.len() != 0 || suffix.len() != 0 {
            panic!("Not support unaligned data");
        }

        Ok(Self { data })
    }
}

impl<'se, const T: usize> Serialize for Matrix<'se, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(self.data.len()))?;
        for x in self.data {
            seq.serialize_element(x)?;
        }
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use super::Matrix;
    use crate::{Dtb, DtbPtr, buildin::Node, from_raw_mut};
    use serde_derive::Serialize;

    const MAX_SIZE: usize = 256;
    #[test]
    fn base_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: [u32; 16],
        }
        let array: [u32; 16] = [0xdeadbeef; 16];
        let mut buf1 = [0u8; MAX_SIZE];

        {
            let base = Base { hello: array };
            crate::ser::to_dtb(&base, &[], &mut buf1).unwrap();
        }

        let ptr = DtbPtr::from_raw(buf1.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();
        let node: Node = from_raw_mut(&dtb).unwrap();
        let matrix = node.get_prop("hello").unwrap().deserialize::<Matrix<4>>();
        let mut count = 0;
        for x in matrix.iter() {
            for y in x {
                count += 1;
                assert_eq!(u32::from_be(*y), 0xdeadbeef);
            }
        }
        assert_eq!(count, 16);
    }
}
