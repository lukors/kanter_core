use std::{
    collections::VecDeque,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    mem::size_of,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};
use tempfile::tempfile;

use crate::{
    error::{Result, TexProError},
    slot_data::{Buffer, ChannelPixel, Size, SlotData},
};

/// A buffer that can be either in memory or in storage, getting it puts it in memory.
#[derive(Debug)]
pub enum TransientBuffer {
    Memory(Box<Buffer>),
    Storage(File, Size),
}

impl TransientBuffer {
    pub fn new(buffer: Box<Buffer>) -> Self {
        Self::Memory(buffer)
    }

    /// Makes sure the `TransientBuffer` is in memory and returns its buffer.
    pub fn buffer(&mut self) -> Result<&Buffer> {
        self.to_memory()?;
        Ok(self.buffer_read().ok_or(TexProError::Generic)?)
    }

    pub fn buffer_read(&self) -> Option<&Buffer> {
        if let Self::Memory(box_buf) = self {
            Some(&box_buf)
        } else {
            None
        }
    }

    pub fn size(&self) -> Size {
        match self {
            Self::Memory(box_buffer) => box_buffer.dimensions().into(),
            Self::Storage(_, size) => *size,
        }
    }

    pub fn bytes(&self) -> usize {
        self.size().pixel_count() * size_of::<ChannelPixel>()
    }

    pub fn in_memory(&self) -> bool {
        match self {
            Self::Memory(_) => true,
            Self::Storage(_, _) => false,
        }
    }

    /// Ensures the `TransientBuffer` is in storage.
    pub fn to_storage(&mut self) -> Result<()> {
        if let Self::Memory(box_buffer) = self {
            let mut file = tempfile()?;

            for pixel in box_buffer.iter() {
                file.write(&pixel.to_ne_bytes())?;
            }

            *self = Self::Storage(file, box_buffer.dimensions().into());
        }

        Ok(())
    }

    /// Ensures the `TransientBuffer` is in memory.
    pub fn to_memory(&mut self) -> Result<()> {
        if let Self::Storage(file, size) = self {
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
        }

        Ok(())
    }
}

/// A container for a `TransientBuffer`. Keeps track of if its `TransientBuffer` has been retrieved.
#[derive(Debug)]
pub struct TransientBufferContainer {
    retrieved: AtomicBool,
    transient_buffer: RwLock<TransientBuffer>,
}

impl TransientBufferContainer {
    pub fn new(transient_buffer: RwLock<TransientBuffer>) -> Self {
        Self {
            retrieved: AtomicBool::new(false),
            transient_buffer,
        }
    }

    pub fn transient_buffer(&self) -> &RwLock<TransientBuffer> {
        self.retrieved.store(true, Ordering::Relaxed);
        &self.transient_buffer
    }

    pub fn size(&self) -> Result<Size> {
        Ok(self.transient_buffer.read()?.size())
    }

    pub fn retrieved(&self) -> bool {
        self.retrieved.load(Ordering::Relaxed)
    }
}

#[derive(Default)]
pub(crate) struct TransientBufferQueue {
    queue: VecDeque<Arc<TransientBufferContainer>>,
    pub memory_limit: usize,
}

impl TransientBufferQueue {
    pub fn new(memory_limit: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            memory_limit,
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

            if self.queue[i].transient_buffer.read()?.in_memory() {
                bytes_in_memory += self.queue[i].transient_buffer.read()?.bytes();
            }

            if self.queue[i].retrieved.swap(false, Ordering::Relaxed) {
                self.queue.swap(i, self.queue.len());
            }
        }

        let mut i: usize = 0;
        while bytes_in_memory > self.memory_limit {
            if let Some(tbuf_container) = self.queue.get(i) {
                tbuf_container.transient_buffer.write()?.to_storage()?;
                bytes_in_memory -= tbuf_container.transient_buffer.read()?.bytes();
            } else {
                return Ok(());
            }

            i += 1;
        }

        Ok(())
    }
}
