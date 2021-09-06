use std::{collections::VecDeque, fmt::{self, Display}, fs::File, io::{Read, Seek, SeekFrom, Write}, mem::size_of, sync::{Arc, RwLock, RwLockReadGuard, atomic::{AtomicBool, Ordering}}, thread, time::Duration};
use tempfile::tempfile;

use crate::{
    error::{Result, TexProError},
    slot_data::{Buffer, ChannelPixel, Size, SlotData},
};

/// A buffer that can be either in memory or in storage, getting it puts it in memory.
#[derive(Debug)]
pub enum TransientBuffer {
    Memory(Box<Buffer>),
    Storage(File, Size, AtomicBool), // Turn the contents of this enum into a struct
}

impl TransientBuffer {
    pub fn new(buffer: Box<Buffer>) -> Self {
        Self::Memory(buffer)
    }

    /// Makes sure the `TransientBuffer` is in memory and returns its buffer.
    pub fn buffer(&mut self) -> Result<&Buffer> {
        self.to_memory()?;
        self.buffer_read()
    }

    pub fn buffer_read(&self) -> Result<&Buffer> {
        if let Self::Memory(box_buf) = self {
            Ok(&box_buf)
        } else {
            Err(TexProError::Generic)
        }
    }

    pub fn buffer_test(&self) -> &Buffer {
        if let Self::Memory(box_buf) = self {
            &box_buf
        } else {
            panic!("This should be unreachable")
        }
    }

    pub fn size(&self) -> Size {
        match self {
            Self::Memory(box_buffer) => box_buffer.dimensions().into(),
            Self::Storage(_, size, _) => *size,
        }
    }

    pub fn bytes(&self) -> usize {
        self.size().pixel_count() * size_of::<ChannelPixel>()
    }

    pub fn request(&self) {
        if let Self::Storage(_, _, requested) = self {
            requested.store(true, Ordering::Relaxed);
        }
    }

    pub fn in_memory(&self) -> bool {
        match self {
            Self::Memory(_) => true,
            Self::Storage(_, _, _) => false,
        }
    }

    /// Ensures the `TransientBuffer` is in storage, returns true if it was moved.
    fn to_storage(&mut self) -> Result<bool> {
        if let Self::Memory(box_buffer) = self {
            let mut file = tempfile()?;

            for pixel in box_buffer.iter() {
                file.write(&pixel.to_ne_bytes())?;
            }

            *self = Self::Storage(file, box_buffer.dimensions().into(), AtomicBool::new(false));

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Ensures the `TransientBuffer` is in memory, returns true if it was moved.
    fn to_memory(&mut self) -> Result<bool> {
        if let Self::Storage(file, size, _) = self {
            let buffer_f32: Vec<f32> = {
                let buffer_int: Vec<u8> = {
                    let mut buffer = Vec::<u8>::new();
                    file.seek(SeekFrom::Start(0))?;
                    file.read_to_end(&mut buffer)?;
                    buffer
                };

                let pixel_count = buffer_int.len() / size_of::<ChannelPixel>();
                let mut buffer = Vec::with_capacity(pixel_count);

                for i in (0..buffer_int.len()).step_by(4) {
                    let bytes: [u8; 4] = [
                        buffer_int[i],
                        buffer_int[i + 1],
                        buffer_int[i + 2],
                        buffer_int[i + 3],
                    ];
                    buffer.push(f32::from_ne_bytes(bytes));
                }

                buffer
            };

            *self = Self::Memory(Box::new(
                Buffer::from_raw(size.width, size.height, buffer_f32)
                    .ok_or(TexProError::Generic)?,
            ));

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// A container for a `TransientBuffer`. Keeps track of if its `TransientBuffer` has been retrieved.
#[derive(Debug)]
pub struct TransientBufferContainer {
    // retrieved: AtomicBool,
    transient_buffer: Arc<RwLock<TransientBuffer>>,
}

impl TransientBufferContainer {
    pub fn new(transient_buffer: Arc<RwLock<TransientBuffer>>) -> Self {
        Self {
            // retrieved: AtomicBool::new(false),
            transient_buffer,
        }
    }

    pub fn test_read(&self) -> RwLockReadGuard<TransientBuffer> {
        loop {
            if let Ok(transient_buffer) = self.transient_buffer.read() {
                if transient_buffer.in_memory() {
                    return transient_buffer
                } else {
                    transient_buffer.request();
                }
            } else {
                panic!("Lock poisoned");
            }

            thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn from_self(&self) -> Self {
        Self {
            // retrieved: AtomicBool::new(false),
            transient_buffer: Arc::clone(&self.transient_buffer),
        }
    }

    pub fn transient_buffer(&self) -> &RwLock<TransientBuffer> {
        // self.retrieved.store(true, Ordering::Relaxed);
        // self.transient_buffer
        //     .write()
        //     .expect("Lock poisoned")
        //     .to_memory()
        //     .expect("Could not move to memory");
        &self.transient_buffer
    }

    pub(crate) fn transient_buffer_sneaky(&self) -> &RwLock<TransientBuffer> {
        &self.transient_buffer
    }

    pub fn size(&self) -> Result<Size> {
        Ok(self.transient_buffer.read()?.size())
    }

    // pub fn retrieved(&self) -> bool {
    //     self.retrieved.load(Ordering::Relaxed)
    // }
}

#[derive(Default)]
pub(crate) struct TransientBufferQueue {
    queue: VecDeque<Arc<TransientBufferContainer>>,
    pub memory_threshold: usize,
}

impl Display for TransientBufferQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes_memory = self.bytes_memory();
        let bytes_storage = self.bytes_storage();
        let bytes_total = bytes_memory + bytes_storage;

        let top = format!(
            "Thres: {thr}\nTotal: {tot}\nStora: {sto}\nMemor: {mem}",
            thr = self.memory_threshold,
            tot = bytes_total,
            mem = bytes_memory,
            sto = bytes_storage
        );

        let queue = self
            .queue
            .iter()
            .map(|arc_tbc| {
                let tbc = arc_tbc.transient_buffer.read().unwrap();
                let location = if tbc.in_memory() { "MEM" } else { "STO" };
                let bytes = tbc.bytes();
                format!("{} {:5} {:p}", location, bytes, *arc_tbc)
            })
            .collect::<Vec<String>>()
            .join("\n");

        write!(f, "{}\n{}", top, queue)
    }
}

impl TransientBufferQueue {
    pub fn new(memory_limit: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            memory_threshold: memory_limit,
        }
    }

    pub fn add_buffer(&mut self, tbuf_container: Arc<TransientBufferContainer>) -> Result<()> {
        if self
            .queue
            .iter()
            .any(|tbc| Arc::ptr_eq(tbc, &tbuf_container))
        {
            return Ok(());
        }

        if tbuf_container.transient_buffer.read()?.in_memory() {
            self.queue.push_back(tbuf_container);
        } else {
            self.queue.push_front(tbuf_container);
        }

        Ok(())
    }

    pub fn add_slot_data(&mut self, slot_data: &Arc<SlotData>) -> Result<()> {
        for buf in slot_data.image.bufs() {
            self.add_buffer(buf)?
        }

        Ok(())
    }

    /// Makes sure this queue is not the only one holding a reference to any `Arc`.
    /// Moves any retrieved `TransientBufferContainer`s to the back of the `queue`.
    /// Also makes sure it stays below its `memory_limit` by moving `TransientBufferContainer`s to
    /// storage from the front of the `queue`.
    pub fn update(&mut self) -> Result<()> {
        let mut bytes_in_memory = 0;

        for i in (0..self.queue.len()).rev() {
            if Arc::strong_count(&self.queue[i]) == 1 {
                self.queue.remove(i);
                continue;
            }

            // if self.queue[i].retrieved.swap(false, Ordering::Relaxed) {
            //     if let Some(removed) = self.queue.remove(i) {
            //         removed.transient_buffer.write()?.to_memory()?;
            //         self.queue.push_back(removed);
            //     }
            // }

            if self.queue[i].transient_buffer.read()?.in_memory() {
                bytes_in_memory += self.queue[i].transient_buffer.read()?.bytes();
            }
        }

        let mut i: usize = 0;
        while bytes_in_memory > self.memory_threshold {
            if let Some(tbuf_container) = self.queue.get(i) {
                let transient_buffer = &tbuf_container.transient_buffer;

                if transient_buffer.write()?.to_storage()? {
                    bytes_in_memory -= transient_buffer.read()?.bytes();
                }
            } else {
                return Ok(());
            }

            i += 1;
        }

        Ok(())
    }

    pub fn bytes_memory(&self) -> usize {
        self.queue
            .iter()
            .map(|tbc| tbc.transient_buffer.read().unwrap())
            .filter(|tb| tb.in_memory())
            .map(|tb| tb.bytes())
            .sum()
    }

    pub fn bytes_storage(&self) -> usize {
        self.queue
            .iter()
            .map(|tbc| tbc.transient_buffer.read().unwrap())
            .filter(|tb| !tb.in_memory())
            .map(|tb| tb.bytes())
            .sum()
    }
}
