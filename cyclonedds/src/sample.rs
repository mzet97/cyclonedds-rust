use cyclonedds_rust_sys::dds_sample_info_t;
use std::ffi::c_void;

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

pub struct Loan<T> {
    samples: Vec<*mut c_void>,
    infos: Vec<dds_sample_info_t>,
    count: usize,
    reader: i32,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Loan<T> {
    pub(crate) fn new(
        mut samples: Vec<*mut c_void>,
        mut infos: Vec<dds_sample_info_t>,
        count: usize,
        reader: i32,
    ) -> Self {
        samples.truncate(count);
        infos.truncate(count);
        Self {
            samples,
            infos,
            count,
            reader,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Sample<&T>> {
        (0..self.count).map(move |i| unsafe {
            Sample {
                data: &*(self.samples[i] as *const T),
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

    pub fn to_vec(&self) -> Vec<Sample<T>>
    where
        T: Clone,
    {
        self.iter()
            .map(|s| Sample {
                data: s.data.clone(),
                info: s.info,
            })
            .collect()
    }
}

impl<T> Drop for Loan<T> {
    fn drop(&mut self) {
        if !self.samples.is_empty() && self.count > 0 {
            unsafe {
                cyclonedds_rust_sys::dds_return_loan(
                    self.reader,
                    self.samples.as_mut_ptr(),
                    self.count as i32,
                );
            }
        }
    }
}
