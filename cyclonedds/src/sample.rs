use cyclonedds_rust_sys::dds_sample_info_t;
use std::ffi::c_void;

use crate::{DdsEntity as _, DdsResult}; // FFI-LIFE-011: Loan::drop chama reader.entity()

pub struct Sample<T> {
    pub data: T,
    pub info: dds_sample_info_t,
}

impl<T> Sample<T> {
    pub fn is_valid(&self) -> bool {
        self.info.valid_data
    }

    pub fn source_timestamp(&self) -> i64 {
        self.info.source_timestamp
    }

    pub fn instance_handle(&self) -> u64 {
        self.info.instance_handle
    }

    pub fn sample_state(&self) -> u32 {
        self.info.sample_state
    }

    pub fn view_state(&self) -> u32 {
        self.info.view_state
    }

    pub fn instance_state(&self) -> u32 {
        self.info.instance_state
    }

    pub fn publication_handle(&self) -> u64 {
        self.info.publication_handle
    }

    pub fn disposed_generation_count(&self) -> u32 {
        self.info.disposed_generation_count
    }

    pub fn no_writers_generation_count(&self) -> u32 {
        self.info.no_writers_generation_count
    }

    pub fn sample_rank(&self) -> u32 {
        self.info.sample_rank
    }

    pub fn generation_rank(&self) -> u32 {
        self.info.generation_rank
    }

    pub fn absolute_generation_rank(&self) -> u32 {
        self.info.absolute_generation_rank
    }
}

pub struct Loan<'a, T: crate::DdsType> {
    samples: Vec<*mut c_void>,
    infos: Vec<dds_sample_info_t>,
    count: usize,
    // FFI-LIFE-011: o loan agora SEGURA uma referência ao reader — o tipo
    // garante que `dds_return_loan` nunca corre num handle morto e que as
    // amostras não são lidas após o reader ser deletado (antes: `reader: i32`
    // cru; o compilador não impedia reader cair antes do loan).
    reader: &'a crate::DataReader<T>,
}

impl<'a, T: crate::DdsType> Loan<'a, T> {
    pub(crate) fn new(
        mut samples: Vec<*mut c_void>,
        mut infos: Vec<dds_sample_info_t>,
        count: usize,
        reader: &'a crate::DataReader<T>,
    ) -> Self {
        samples.truncate(count);
        infos.truncate(count);
        Self {
            samples,
            infos,
            count,
            reader,
        }
    }

    /// Iterate over the loaned samples as owned `T` values.
    ///
    /// The loaned memory holds `T::Native`, *not* `T` — for a type with
    /// `String`/`Vec` fields those are `DdsString` (8 bytes) / `DdsSequence` on
    /// the wire, where `T` expects `String` (24 bytes) / `Vec`. Handing out
    /// `&T` over that buffer (as this method used to do) is an out-of-bounds
    /// read, and cloning through such a reference corrupts the heap: the
    /// resulting `String` carries a garbage capacity/length and is later freed
    /// by Rust over memory it never allocated. Each sample is therefore
    /// converted with [`crate::DdsType::clone_out`], the same path already used by
    /// `read`/`take`/`read_next`.
    ///
    /// For a genuinely zero-copy view, use [`iter_native`](Self::iter_native).
    pub fn iter(&self) -> impl Iterator<Item = DdsResult<Sample<T>>> + '_ {
        (0..self.count).map(move |i| unsafe {
            Ok(Sample {
                data: T::clone_out(self.samples[i] as *const T)?,
                info: self.infos[i],
            })
        })
    }

    /// Zero-copy view over the loaned samples, borrowing the DDS-owned buffers
    /// directly as [`crate::DdsType::Native`].
    ///
    /// This is the wire representation: for a type with `String`/`Vec` fields
    /// you get `DdsString`/`DdsSequence`, not `String`/`Vec`. Nothing is
    /// copied, so this is the path to use when the sample is large and only a
    /// few fields are read. The borrow ends when the `Loan` is dropped.
    pub fn iter_native(&self) -> impl Iterator<Item = Sample<&T::Native>> + '_ {
        (0..self.count).map(move |i| unsafe {
            Sample {
                data: &*(self.samples[i] as *const T::Native),
                info: self.infos[i],
            }
        })
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Copy every loaned sample out into owned values.
    ///
    /// No longer requires `T: Clone` — the conversion goes through
    /// [`crate::DdsType::clone_out`], which is the only sound way to read the native
    /// buffer (see [`iter`](Self::iter)).
    pub fn to_vec(&self) -> DdsResult<Vec<Sample<T>>> {
        self.iter().collect()
    }
}

impl<T: crate::DdsType> Drop for Loan<'_, T> {
    fn drop(&mut self) {
        if !self.samples.is_empty() && self.count > 0 {
            unsafe {
                cyclonedds_rust_sys::dds_return_loan(
                    self.reader.entity(),
                    self.samples.as_mut_ptr(),
                    self.count as i32,
                );
            }
        }
    }
}
