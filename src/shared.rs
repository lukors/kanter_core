use crate::error::{Result, TexProError};
use crate::{node::*, node_data::*};
use image::{imageops, DynamicImage, FilterType, GenericImageView, ImageBuffer};
use std::{
    cmp::{max, min},
    path::Path,
    sync::Arc,
    u32,
};

pub fn has_dup<T: PartialEq>(slice: &[T]) -> bool {
    for i in 1..slice.len() {
        if slice[i..].contains(&slice[i - 1]) {
            return true;
        }
    }
    false
}

pub fn channels_to_rgba(channels: &[Arc<Buffer>]) -> Result<Vec<u8>> {
    if channels.len() != 4 {
        return Err(TexProError::InvalidBufferCount);
    }

    fn clamp_float(input: f32) -> f32 {
        if input < 0. {
            return 0.
        } else if input > 1. {
            return 1.
        } else {
            return input
        }
    }

    Ok(channels[0]
        .pixels()
        .zip(channels[1].pixels())
        .zip(channels[2].pixels())
        .zip(channels[3].pixels())
        .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
        .flatten()
        .map(|x| (clamp_float(x[0]) * 255.).min(255.) as u8)
        .collect())
}

pub fn channels_to_rgba_arc(channels: &[&Buffer]) -> Result<Vec<u8>> {
    if channels.len() != 4 {
        return Err(TexProError::InvalidBufferCount);
    }

    Ok(channels[0]
        .pixels()
        .zip(channels[1].pixels())
        .zip(channels[2].pixels())
        .zip(channels[3].pixels())
        .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
        .flatten()
        .map(|x| (x[0] * 255.).min(255.) as u8)
        .collect())
}

pub fn deconstruct_image(image: &DynamicImage) -> Vec<Buffer> {
    let raw_pixels = image.raw_pixels();
    let (width, height) = (image.width(), image.height());
    let pixel_count = (width * height) as usize;
    let channel_count = raw_pixels.len() / pixel_count;
    let max_channel_count = 4;
    let mut pixel_vecs: Vec<Vec<f32>> = Vec::with_capacity(max_channel_count);

    for _ in 0..max_channel_count {
        pixel_vecs.push(Vec::with_capacity(pixel_count));
    }

    let mut current_channel = 0;

    for component in raw_pixels {
        pixel_vecs[current_channel].push(ChannelPixel::from(component) / 255.);
        current_channel = (current_channel + 1) % channel_count;
    }

    for (i, item) in pixel_vecs
        .iter_mut()
        .enumerate()
        .take(max_channel_count)
        .skip(channel_count)
    {
        *item = match i {
            3 => vec![1.; pixel_count],
            _ => vec![0.; pixel_count],
        }
    }

    pixel_vecs
        .into_iter()
        .map(|p_vec| {
            Box::new(
                ImageBuffer::from_raw(width, height, p_vec)
                    .expect("A bug in the deconstruct_image function caused a crash"),
            )
        })
        .collect()
}

pub fn resize_buffers(
    node_datas: &[Arc<NodeData>],
    policy: Option<ResizePolicy>,
    filter: Option<FilterType>,
) -> Result<Vec<Arc<NodeData>>> {
    let policy = policy.unwrap_or(ResizePolicy::LargestAxes);
    let filter = filter.unwrap_or(FilterType::Triangle);

    let size = match policy {
        ResizePolicy::MostPixels => node_datas
            .iter()
            .max_by(|a, b| a.size.pixel_count().cmp(&b.size.pixel_count()))
            .map(|node_data| node_data.size)
            .unwrap(),
        ResizePolicy::LeastPixels => node_datas
            .iter()
            .min_by(|a, b| a.size.pixel_count().cmp(&b.size.pixel_count()))
            .map(|node_data| node_data.size)
            .unwrap(),
        ResizePolicy::LargestAxes => node_datas.iter().fold(Size::new(0, 0), |a, b| {
            Size::new(max(a.width, b.size.width), max(a.height, b.size.height))
        }),
        ResizePolicy::SmallestAxes => node_datas
            .iter()
            .fold(Size::new(u32::MAX, u32::MAX), |a, b| {
                Size::new(min(a.width, b.size.width), min(a.height, b.size.height))
            }),
        ResizePolicy::SpecificSlot(slot_id) => {
            node_datas
                .iter()
                .find(|node_data| node_data.slot_id == slot_id)
                .expect("Couldn't find a buffer with the given `NodeId` while resizing")
                .size
        }
        ResizePolicy::SpecificSize(size) => size,
    };

    let output: Vec<Arc<NodeData>> = node_datas
        .iter()
        // .filter(|ref node_data| node_data.size != size)
        .map(|ref node_data| {
            if node_data.size != size {
                // Needs to be resized
                let resized_buffer: Arc<Buffer> = Arc::new(Box::new(imageops::resize(
                    &**node_data.buffer,
                    size.width,
                    size.height,
                    filter,
                )));
                Arc::new(NodeData::new(
                    node_data.node_id,
                    node_data.slot_id,
                    node_data.size,
                    Arc::clone(&resized_buffer),
                ))
            } else {
                // Does not need to be resized
                Arc::clone(node_data)
            }
        })
        .collect();

    Ok(output)
}

pub fn read_image<P: AsRef<Path>>(path: P) -> Result<Vec<Buffer>> {
    let image = image::open(path)?;
    let buffers = deconstruct_image(&image);

    Ok(buffers)
}

pub fn write_image<P: AsRef<Path>>(inputs: &[Arc<NodeData>], path: P) -> Result<()> {
    let channel_vec: Vec<Arc<Buffer>> = inputs
        .iter()
        .map(|node_data| Arc::clone(&node_data.buffer))
        .collect();
    let (width, height) = (inputs[0].size.width, inputs[0].size.height);
    let img = {
        if let Some(img) =
            image::RgbaImage::from_vec(width, height, channels_to_rgba(&channel_vec)?)
        {
            img
        } else {
            return Err(TexProError::InconsistentVectorLengths);
        }
    };

    image::save_buffer(path, &img, width, height, image::ColorType::RGBA(8))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::shared::NodeId;

    fn buffers_equal(buf_1: &Buffer, buf_2: &Buffer) -> bool {
        if buf_1.len() != buf_2.len() {
            return false;
        }

        !buf_1.pixels().zip(buf_2.pixels()).any(|(a, b)| a != b)
    }

    fn images_equal(img_1: &DynamicImage, img_2: &DynamicImage) -> bool {
        let bufs_1 = deconstruct_image(&img_1);
        let bufs_2 = deconstruct_image(&img_2);

        !bufs_1
            .iter()
            .zip(&bufs_2)
            .any(|(a, b)| !buffers_equal(a, b))
    }

    fn images_equal_path<P: AsRef<Path>>(path_1: P, path_2: P) -> bool {
        let bufs_1 = deconstruct_image(
            &image::open(path_1).expect("Unable to open image at path_1 to compare it"),
        );
        let bufs_2 = deconstruct_image(
            &image::open(path_2).expect("Unable to open image at path_2 to compare it"),
        );

        !bufs_1
            .iter()
            .zip(&bufs_2)
            .any(|(a, b)| !buffers_equal(a, b))
    }

    fn buffer_vecs_equal(bufs_1: &[Buffer], bufs_2: &[Buffer]) -> bool {
        if bufs_1.len() != bufs_2.len() {
            return false;
        }

        !bufs_1
            .iter()
            .zip(bufs_2.iter())
            .any(|(a, b)| !buffers_equal(a, b))
    }

    // fn detached_buffers_equal(bufs_1: &[DetachedBuffer], bufs_2: &[DetachedBuffer]) -> bool {
    //     if bufs_1.len() != bufs_2.len() {
    //         return false;
    //     }

    //     !bufs_1
    //         .iter()
    //         .zip(bufs_2.iter())
    //         .any(|(a, b)| !buffers_equal(&a.buffer(), &b.buffer()))
    // }

    // #[test]
    // fn resize_buffers_policy_specific_size() {
    //     let input_path = Path::new(&"data/heart_128.png");

    //     let mut buffers = read_image(&input_path).unwrap();
    //     resize_buffers(
    //         &mut buffers,
    //         Some(ResizePolicy::SpecificSize(Size::new(256, 256))),
    //         None,
    //     )
    //     .unwrap();

    //     let target_size = Size::new(256, 256);
    //     let target_buffer_length = 256 * 256;
    //     for buffer in buffers {
    //         assert_eq!(buffer.buffer().len(), target_buffer_length);
    //         assert_eq!(buffer.size(), target_size);
    //     }
    // }

    // #[test]
    // fn resize_buffers_policy_most_pixels() {
    //     let input_1_path = Path::new(&"data/heart_128.png");
    //     let input_2_path = Path::new(&"data/heart_256.png");

    //     let mut buffers = read_image(&input_2_path).unwrap();
    //     let target_buffer_length = buffers[0].buffer().len();
    //     buffers.append(&mut read_image(&input_1_path).unwrap());

    //     resize_buffers(&mut buffers, Some(ResizePolicy::MostPixels), None).unwrap();

    //     let target_size = Size::new(256, 256);
    //     for buffer in buffers {
    //         assert_eq!(buffer.buffer().len(), target_buffer_length);
    //         assert_eq!(buffer.size(), target_size);
    //     }
    // }

    // #[test]
    // fn resize_buffers_policy_least_pixels() {
    //     let input_1_path = Path::new(&"data/heart_128.png");
    //     let input_2_path = Path::new(&"data/heart_256.png");

    //     let mut buffers = read_image(&input_1_path).unwrap();
    //     let target_buffer_length = buffers[0].buffer().len();
    //     buffers.append(&mut read_image(&input_2_path).unwrap());

    //     resize_buffers(&mut buffers, Some(ResizePolicy::LeastPixels), None).unwrap();

    //     let target_size = Size::new(128, 128);
    //     for buffer in buffers {
    //         assert_eq!(buffer.buffer().len(), target_buffer_length);
    //         assert_eq!(buffer.size(), target_size);
    //     }
    // }

    // #[test]
    // fn resize_buffers_policy_largest_axes() {
    //     let input_1_path = Path::new(&"data/heart_wide.png");
    //     let input_2_path = Path::new(&"data/heart_tall.png");

    //     let mut buffers = read_image(&input_1_path).unwrap();
    //     buffers.append(&mut read_image(&input_2_path).unwrap());
    //     let target_buffer_length = buffers[0].buffer().len() * 2;

    //     resize_buffers(&mut buffers, Some(ResizePolicy::LargestAxes), None).unwrap();

    //     let target_size = Size::new(128, 128);
    //     for buffer in buffers {
    //         assert_eq!(buffer.buffer().len(), target_buffer_length);
    //         assert_eq!(buffer.size(), target_size);
    //     }
    // }

    // #[test]
    // fn resize_buffers_policy_smallest_axes() {
    //     let input_1_path = Path::new(&"data/heart_wide.png");
    //     let input_2_path = Path::new(&"data/heart_tall.png");

    //     let mut buffers = read_image(&input_1_path).unwrap();
    //     buffers.append(&mut read_image(&input_2_path).unwrap());
    //     let target_buffer_length = buffers[0].buffer().len() / 2;

    //     resize_buffers(&mut buffers, Some(ResizePolicy::SmallestAxes), None).unwrap();

    //     let target_size = Size::new(64, 64);
    //     for buffer in buffers {
    //         assert_eq!(buffer.buffer().len(), target_buffer_length);
    //         assert_eq!(buffer.size(), target_size);
    //     }
    // }

    // #[test]
    // fn resize_buffers_policy_specific_node() {
    //     let input_1_path = Path::new(&"data/heart_128.png");
    //     let input_2_path = Path::new(&"data/heart_256.png");

    //     let mut buffers_1 = read_image(&input_1_path).unwrap();
    //     for mut buffer in &mut buffers_1 {
    //         buffer.set_id(Some(NodeId::new(1)));
    //     }
    //     let target_buffer_length = buffers_1[0].buffer().len();

    //     let mut buffers_2 = read_image(&input_2_path).unwrap();
    //     for mut buffer in &mut buffers_2 {
    //         buffer.set_id(Some(NodeId::new(2)));
    //     }

    //     buffers_1.append(&mut buffers_2);

    //     resize_buffers(
    //         &mut buffers_1,
    //         Some(ResizePolicy::SpecificNode(NodeId::new(1))),
    //         None,
    //     )
    //     .unwrap();

    //     let target_size = Size::new(128, 128);
    //     for buffer in buffers_1 {
    //         assert_eq!(buffer.buffer().len(), target_buffer_length);
    //         assert_eq!(buffer.size(), target_size);
    //     }
    // }
}
