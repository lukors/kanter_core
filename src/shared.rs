extern crate image;

use self::image::{DynamicImage, FilterType, GenericImageView, ImageBuffer, imageops};
use node::{Buffer, ChannelPixel, DetachedBuffer, ResizePolicy, Size, Slot};
use std::{
    cmp::{max, min},
    error::Error,
    fmt::{self, Display, Formatter},
    path::Path,
    result,
    sync::Arc,
    u32,
};

type Result<T> = result::Result<T, TexProError>;

#[derive(Debug, Clone)]
pub enum TexProError {
    GenericError,
}


impl Display for TexProError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            TexProError::GenericError => f.write_str("GenericError"),
        }
    }
}

impl Error for TexProError {
    fn description(&self) -> &str {
        match *self {
            TexProError::GenericError => "Unspecified error",
        }
    }
}

pub fn channels_to_rgba(channels: &[&Buffer]) -> Vec<u8> {
    if channels.len() != 4 {
        panic!("The number of channels when converting to an RGBA image needs to be 4");
    }

    channels[0]
        .pixels()
        .zip(channels[1].pixels())
        .zip(channels[2].pixels())
        .zip(channels[3].pixels())
        .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
        .flatten()
        .map(|x| (x[0] * 255.).min(255.) as u8)
        .collect()
}

pub fn channels_to_rgba_arc(channels: &[Arc<Buffer>]) -> Vec<u8> {
    if channels.len() != 4 {
        panic!("The number of channels when converting to an RGBA image needs to be 4");
    }

    channels[0]
        .pixels()
        .zip(channels[1].pixels())
        .zip(channels[2].pixels())
        .zip(channels[3].pixels())
        .map(|(((r, g), b), a)| vec![r, g, b, a].into_iter())
        .flatten()
        .map(|x| (x[0] * 255.).min(255.) as u8)
        .collect()
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

    for (i, mut item) in pixel_vecs
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
        .map(|p_vec| ImageBuffer::from_raw(width, height, p_vec).unwrap())
        .collect()
}

pub fn resize_buffers(
    buffers: &mut [DetachedBuffer],
    policy: Option<ResizePolicy>,
    filter: Option<FilterType>,
) {
    if buffers.len() < 2 {
        return;
    }

    let policy = policy.unwrap_or(ResizePolicy::LargestAxes);
    let filter = filter.unwrap_or(FilterType::Triangle);

    let size = match policy {
        ResizePolicy::MostPixels => buffers
            .iter()
            .max_by(|a, b| a.size().pixel_count().cmp(&b.size().pixel_count()))
            .map(|buffer| buffer.size())
            .unwrap(),
        ResizePolicy::LeastPixels => buffers
            .iter()
            .min_by(|a, b| a.size().pixel_count().cmp(&b.size().pixel_count()))
            .map(|buffer| buffer.size())
            .unwrap(),
        ResizePolicy::LargestAxes => buffers
            .iter()
            .fold(Size::new(0, 0), |a, b| {
                Size::new(
                    max(a.width(), b.size().width()),
                    max(a.height(), b.size().height()),
                )
            }),
        ResizePolicy::SmallestAxes => buffers
            .iter()
            .fold(Size::new(u32::MAX, u32::MAX), |a, b| {
                Size::new(
                    min(a.width(), b.size().width()),
                    min(a.height(), b.size().height()),
                )
            }),
        ResizePolicy::SpecificNode(node_id) => buffers
            .iter()
            .find(|buffer| buffer.id() == Some(node_id) )
            .expect("Couldn't find a buffer with the given `NodeId` while resizing")
            .size(),
        ResizePolicy::SpecificSize(size) => size,
    };

    buffers
        .iter_mut()
        .filter(|ref buffer| buffer.size() != size)
        .for_each(|ref mut buffer| {
            let resized_buffer = imageops::resize(
                &*buffer.buffer(),
                size.width(),
                size.height(),
                filter,
            );
            buffer.set_buffer(resized_buffer);
            buffer.set_size(size);
        });
}

pub fn read_image(path: &Path) -> Vec<DetachedBuffer> {
    let image = image::open(path).expect("Could not open the given image");
    let buffers = deconstruct_image(&image);

    let mut output = Vec::new();

    for (channel, buffer) in buffers.into_iter().enumerate() {
        output.push(DetachedBuffer::new(
            None,
            Slot(channel),
            Size::new(image.width(), image.height()),
            Arc::new(buffer),
        ));
    }

    output
}

pub fn write_image(inputs: &[DetachedBuffer], path: &Path) {
    let channel_vec: Vec<Arc<Buffer>> = inputs.iter().map(|node_data| node_data.buffer()).collect();
    let (width, height) = (inputs[0].size().width(), inputs[0].size().height());

    image::save_buffer(
        &Path::new(path),
        &image::RgbaImage::from_vec(width, height, channels_to_rgba_arc(&channel_vec)).unwrap(),
        width,
        height,
        image::ColorType::RGBA(8),
    ).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use node::NodeId;

    fn buffers_equal(buf_1: &Buffer, buf_2: &Buffer) -> bool {
        if buf_1.len() != buf_2.len() {
            return false
        }

        !buf_1.pixels()
            .zip(buf_2.pixels())
            .any(|(a, b)| a != b)
    }

    fn images_equal(img_1: &DynamicImage, img_2: &DynamicImage) -> bool {
        let bufs_1 = deconstruct_image(&img_1);
        let bufs_2 = deconstruct_image(&img_2);

        !bufs_1.iter()
            .zip(&bufs_2)
            .any(|(a, b)| !buffers_equal(a, b))
    }

    fn images_equal_path(path_1: &Path, path_2: &Path) -> bool {
        let bufs_1 = deconstruct_image(&image::open(path_1).expect("Unable to open image at path_1 to compare it"));
        let bufs_2 = deconstruct_image(&image::open(path_2).expect("Unable to open image at path_2 to compare it"));

        !bufs_1.iter()
            .zip(&bufs_2)
            .any(|(a, b)| !buffers_equal(a, b))
    }

    fn buffer_vecs_equal(bufs_1: &[Buffer], bufs_2: &[Buffer]) -> bool {
        if bufs_1.len() != bufs_2.len() {
            return false
        }

        !bufs_1.iter()
            .zip(bufs_2.iter())
            .any(|(a, b)| !buffers_equal(a, b))
    }

    fn detached_buffers_equal(bufs_1: &[DetachedBuffer], bufs_2: &[DetachedBuffer]) -> bool {
        if bufs_1.len() != bufs_2.len() {
            return false
        }

        !bufs_1.iter()
            .zip(bufs_2.iter())
            .any(|(a, b)| !buffers_equal(&a.buffer(), &b.buffer()))
    }

    #[test]
    fn resize_buffers_policy_specific_size() {
        let input_path = Path::new(&"data/heart_128.png");

        let mut buffers = read_image(&input_path);
        resize_buffers(&mut buffers, Some(ResizePolicy::SpecificSize(Size::new(256, 256))), None);

        let target_size = Size::new(256, 256);
        let target_buffer_length = 256 * 256;
        for buffer in buffers {
            assert_eq!(buffer.buffer().len(), target_buffer_length);
            assert_eq!(buffer.size(), target_size);
        }
    }

    #[test]
    fn resize_buffers_policy_most_pixels() {
        let input_1_path = Path::new(&"data/heart_128.png");
        let input_2_path = Path::new(&"data/heart_256.png");

        let mut buffers = read_image(&input_2_path);
        let target_buffer_length = buffers[0].buffer().len();
        buffers.append(&mut read_image(&input_1_path));

        resize_buffers(&mut buffers, Some(ResizePolicy::MostPixels), None);

        let target_size = Size::new(256, 256);
        for buffer in buffers {
            assert_eq!(buffer.buffer().len(), target_buffer_length);
            assert_eq!(buffer.size(), target_size);
        }
    }

    #[test]
    fn resize_buffers_policy_least_pixels() {
        let input_1_path = Path::new(&"data/heart_128.png");
        let input_2_path = Path::new(&"data/heart_256.png");

        let mut buffers = read_image(&input_1_path);
        let target_buffer_length = buffers[0].buffer().len();
        buffers.append(&mut read_image(&input_2_path));

        resize_buffers(&mut buffers, Some(ResizePolicy::LeastPixels), None);

        let target_size = Size::new(128, 128);
        for buffer in buffers {
            assert_eq!(buffer.buffer().len(), target_buffer_length);
            assert_eq!(buffer.size(), target_size);
        }
    }

    #[test]
    fn resize_buffers_policy_largest_axes() {
        let input_1_path = Path::new(&"data/heart_wide.png");
        let input_2_path = Path::new(&"data/heart_tall.png");

        let mut buffers = read_image(&input_1_path);
        buffers.append(&mut read_image(&input_2_path));
        let target_buffer_length = buffers[0].buffer().len()*2;

        resize_buffers(&mut buffers, Some(ResizePolicy::LargestAxes), None);

        let target_size = Size::new(128, 128);
        for buffer in buffers {
            assert_eq!(buffer.buffer().len(), target_buffer_length);
            assert_eq!(buffer.size(), target_size);
        }
    }

    #[test]
    fn resize_buffers_policy_smallest_axes() {
        let input_1_path = Path::new(&"data/heart_wide.png");
        let input_2_path = Path::new(&"data/heart_tall.png");

        let mut buffers = read_image(&input_1_path);
        buffers.append(&mut read_image(&input_2_path));
        let target_buffer_length = buffers[0].buffer().len()/2;

        resize_buffers(&mut buffers, Some(ResizePolicy::SmallestAxes), None);

        let target_size = Size::new(64, 64);
        for buffer in buffers {
            assert_eq!(buffer.buffer().len(), target_buffer_length);
            assert_eq!(buffer.size(), target_size);
        }
    }

    #[test]
    fn resize_buffers_policy_specific_node() {
        let input_1_path = Path::new(&"data/heart_128.png");
        let input_2_path = Path::new(&"data/heart_256.png");

        let mut buffers_1 = read_image(&input_1_path);
        for mut buffer in &mut buffers_1 {
            buffer.set_id(Some(NodeId::new(1)));
        }
        let target_buffer_length = buffers_1[0].buffer().len();

        let mut buffers_2 = read_image(&input_2_path);
        for mut buffer in &mut buffers_2 {
            buffer.set_id(Some(NodeId::new(2)));
        }

        buffers_1.append(&mut buffers_2);

        resize_buffers(&mut buffers_1, Some(ResizePolicy::SpecificNode(NodeId::new(1))), None);

        let target_size = Size::new(128, 128);
        for buffer in buffers_1 {
            assert_eq!(buffer.buffer().len(), target_buffer_length);
            assert_eq!(buffer.size(), target_size);
        }
    }
}